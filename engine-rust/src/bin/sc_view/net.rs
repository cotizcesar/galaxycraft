//! `impl App`: net (moved out of `sc_view.rs` by scripts/split_impl.py).

use super::*;

impl App {
    // ------------------------------------------------------------------
    // LAN lobby + lockstep match
    // ------------------------------------------------------------------

    /// Ensure the discovery socket exists and refresh the lobby list.
    pub(super) fn lan_scan(&mut self) {
        let sock = match discovery_socket(galaxycraft::net::DISCOVERY_PORT) {
            Ok(s) => s,
            // Someone else on this machine holds the well-known port: fall
            // back to an ephemeral one so the screen still works.
            Err(_) => match discovery_socket(0) {
                Ok(s) => s,
                Err(e) => {
                    self.lan.status = format!("discovery unavailable: {e}");
                    return;
                }
            },
        };
        match discover(&sock, Duration::from_millis(300)) {
            Ok(list) => {
                self.lan.entries = list;
                self.lan.sel = self.lan.sel.min(self.lan.entries.len().saturating_sub(1));
                self.lan.status = if self.lan.entries.is_empty() {
                    "no games found (H to host)".to_string()
                } else {
                    format!("{} game(s) found", self.lan.entries.len())
                };
            }
            Err(e) => self.lan.status = format!("discovery error: {e}"),
        }
    }

    /// Start hosting: bind the lobby and announce it.  The lobby holds as
    /// many players as the map has start locations (2..=8).
    /// Lockstep turns per network turn from the Network tab's latency mode.
    pub(super) fn latency_turns(&self) -> u32 {
        match self.game_options.network_latency {
            1 => 6,
            2 => 8,
            _ => galaxycraft::net::DEFAULT_TICKS_PER_TURN,
        }
    }

    pub(super) fn lan_host(&mut self) {
        let entry = match self.maps.get(self.lan.map_index) {
            Some(e) => e.clone(),
            None => return,
        };
        let raw = match std::fs::read(&entry.path) {
            Ok(r) => r,
            Err(e) => {
                self.lan.status = format!("cannot read map: {e}");
                return;
            }
        };
        let mut map_hash = [0u8; 32];
        map_hash.copy_from_slice(&Sha256::digest(&raw));
        // Network tab "Prefer Port 6112": bind the retail port instead of
        // this client's historical 7777.
        let port = if self.game_options.network_prefer_6112 {
            6112
        } else {
            galaxycraft::net::LOBBY_PORT
        };
        let addr = format!("0.0.0.0:{port}");
        match Lobby::bind(&addr, map_hash, galaxycraft::net::WORLD_SEED) {
            Ok(mut l) => {
                l.ticks_per_turn = self.latency_turns();
                let queue = galaxycraft::net::JoinQueue::new(entry.players.clamp(2, 8) as u8);
                self.lan.status = format!(
                    "hosting {} on :{port} - 1/{} players",
                    entry.name,
                    queue.capacity()
                );
                self.lan.listener = Some(l);
                self.lan.queue = Some(queue);
                self.lan.announce_sock = announcer().ok();
                self.lan.last_announce = Instant::now() - Duration::from_secs(5);
            }
            Err(e) => self.lan.status = format!("cannot host: {e}"),
        }
    }

    /// While hosting: keep announcing, take joins without blocking, and
    /// start when the lobby is full.
    pub(super) fn lan_poll_host(&mut self) {
        let (Some(lobby), Some(queue)) = (self.lan.listener.as_ref(), self.lan.queue.as_mut())
        else {
            return;
        };
        if self.lan.last_announce.elapsed() >= Duration::from_secs(1) {
            if let Some(sock) = self.lan.announce_sock.as_ref() {
                let a = Announce {
                    name: self
                        .maps
                        .get(self.lan.map_index)
                        .map(|m| m.name.clone())
                        .unwrap_or_default(),
                    map_hash: lobby.map_hash,
                    port: lobby
                        .listener
                        .local_addr()
                        .map(|a| a.port())
                        .unwrap_or(galaxycraft::net::LOBBY_PORT),
                    players: queue.players(),
                    started: false,
                    game_type: galaxycraft::multiplayer::GameType::Melee.tag(),
                    map_name: String::new(),
                    map_size: (0, 0),
                    speed: 6,
                };
                let _ = broadcast(sock, &a);
            }
            self.lan.last_announce = Instant::now();
        }
        match lobby.poll_join(queue, false) {
            Ok(Some(addr)) => {
                self.lan.status = format!(
                    "{addr} joined - {}/{} players (ENTER: start)",
                    queue.players(),
                    queue.capacity()
                );
                if queue.players() == queue.capacity() {
                    self.lan_start_hosted();
                }
            }
            Ok(None) => {}
            Err(e) => self.lan.status = format!("join refused: {e}"),
        }
    }

    /// Close the hosted lobby and start the match with everyone in it.
    pub(super) fn lan_start_hosted(&mut self) {
        let Some(queue) = self.lan.queue.take() else {
            return;
        };
        if queue.players() < 2 {
            self.lan.status = "need at least one other player".to_string();
            self.lan.queue = Some(queue);
            return;
        }
        let Some(lobby) = self.lan.listener.take() else {
            return;
        };
        self.lan.announce_sock = None;
        let players = queue.players();
        match lobby.start(queue, self.menu_race) {
            Ok((clients, races)) => {
                let index = self.lan.map_index;
                let map_hash = lobby.map_hash;
                if self.start_net_match(
                    NetRole::Host,
                    clients,
                    races,
                    index,
                    0,
                    map_hash,
                    Some(lobby),
                    String::new(),
                    None,
                ) {
                    self.lan.status = format!("match started with {players} players");
                }
            }
            Err(e) => self.lan.status = format!("start failed: {e}"),
        }
    }

    /// Join the selected lobby.
    pub(super) fn lan_join(&mut self) {
        let Some((addr, announce)) = self.lan.entries.get(self.lan.sel).cloned() else {
            return;
        };
        let host_addr = format!("{}:{}", addr.ip(), announce.port);
        match galaxycraft::net::connect(
            &host_addr,
            announce.map_hash,
            galaxycraft::net::WORLD_SEED,
            self.menu_race,
        ) {
            Ok((
                t,
                Msg::Setup {
                    races,
                    you,
                    ticks_per_turn,
                    ..
                },
            )) => {
                // The host decides the map and races; find it by hash.
                let index = self
                    .maps
                    .iter()
                    .position(|m| {
                        std::fs::read(&m.path)
                            .map(|raw| {
                                let mut h = [0u8; 32];
                                h.copy_from_slice(&Sha256::digest(&raw));
                                h == announce.map_hash
                            })
                            .unwrap_or(false)
                    })
                    .unwrap_or(self.lan.map_index);
                if self.start_net_match(
                    NetRole::Client,
                    vec![t],
                    races,
                    index,
                    you,
                    announce.map_hash,
                    None,
                    host_addr,
                    None,
                ) {
                    if let Some(net) = &mut self.net {
                        net.lock.ticks_per_turn = ticks_per_turn.max(1);
                    }
                    self.lan.status = format!("joined {addr} as player {}", you + 1);
                }
            }
            Ok((_, other)) => self.lan.status = format!("unexpected: {other:?}"),
            Err(e) => self.lan.status = format!("join failed: {e}"),
        }
    }

    /// Build the world for a networked match and switch into lockstep.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn start_net_match(
        &mut self,
        role: NetRole,
        mut peers: Vec<TcpTransport>,
        races: Vec<Race>,
        map_index: usize,
        local: u8,
        map_hash: [u8; 32],
        lobby: Option<Lobby>,
        host_addr: String,
        setup: Option<galaxycraft::multiplayer::GameSetup>,
    ) -> bool {
        let Some(entry) = self.maps.get(map_index).cloned() else {
            self.lan.status = "no map selected".to_string();
            return false;
        };
        let raw = match std::fs::read(&entry.path) {
            Ok(r) => r,
            Err(e) => {
                self.lan.status = format!("cannot read map: {e}");
                return false;
            }
        };
        // A lobby game seats each peer in its slot; the legacy LAN screen
        // seats peer n in slot n.
        let local_slot = setup
            .as_ref()
            .and_then(|s| s.slot_of_peer(local))
            .unwrap_or(local as usize);
        let races = setup.as_ref().map(|s| s.races()).unwrap_or(races);
        let save = SaveFile {
            map_name: entry.name.clone(),
            map_index: Some(map_index),
            map_chk: raw,
            races: races.clone(),
            local_player: local_slot,
            tick: 0,
            rec: Recording::new(galaxycraft::net::WORLD_SEED, [0u8; 32]),
            setup,
        };
        if !self.build_from_save(&save) {
            return false;
        }
        self.chat_log.clear();
        self.chat_input.clear();
        self.chat_active = false;
        let peer_count = save
            .setup
            .as_ref()
            .map(|s| {
                s.slots
                    .iter()
                    .filter(|x| {
                        matches!(
                            x.occupant,
                            galaxycraft::multiplayer::SetupOccupant::Human { .. }
                        )
                    })
                    .count() as u8
            })
            .unwrap_or(races.len() as u8);
        let ticks_per_turn = lobby
            .as_ref()
            .map_or(galaxycraft::net::DEFAULT_TICKS_PER_TURN, |l| {
                l.ticks_per_turn
            });
        // Computers run inside the lockstep on every peer, not locally.
        let computers = if save.setup.is_some() {
            std::mem::take(&mut self.ais)
        } else {
            Vec::new()
        };
        for p in &mut peers {
            let _ = p.stream_timeout(NET_READ_TIMEOUT);
        }
        self.map_index = map_index;
        self.menu = false;
        self.intro = false;
        self.briefing = false;
        self.paused = false;
        self.speed = 1.0;
        self.accumulator = 0.0;
        self.last_step = Instant::now();
        self.net = Some(NetSession {
            role,
            lock: {
                let mut lock = Lockstep::new(local, peer_count, ticks_per_turn);
                lock.computers = computers;
                lock
            },
            peers,
            desync: false,
            last_hash: 0,
            save,
            map_hash,
            lobby,
            host_addr,
            waiting: None,
            reconnect: None,
        });
        true
    }

    /// Host waiting for a dropped peer: accept rejoins (any peer may come
    /// back) without blocking; give up after [`REJOIN_WAIT`].
    pub(super) fn net_wait_for_rejoin(&mut self, net: &mut NetSession) -> bool {
        let Some((peer, deadline)) = net.waiting else {
            return true;
        };
        let Some(lobby) = net.lobby.as_ref() else {
            return false;
        };
        match galaxycraft::net::try_accept_rejoin(lobby, &net.lock, false) {
            Ok(Some((back, mut t))) => {
                let _ = t.stream_timeout(NET_READ_TIMEOUT);
                net.lock.forget_peer_turn(back);
                if let Some(slot) = net.peers.get_mut(back as usize - 1) {
                    *slot = t;
                }
                if back == peer {
                    net.waiting = None;
                    self.status(format!("player {} rejoined", peer + 1));
                }
                self.last_step = Instant::now();
            }
            Ok(None) => {}
            Err(e) => self.status(format!("rejoin refused: {e}")),
        }
        if net.waiting.is_some() && Instant::now() >= deadline {
            self.status(format!("player {} did not return - match ended", peer + 1));
            return false;
        }
        true
    }

    /// Client that lost the host: reconnect on the backoff schedule, then
    /// rebuild the world from the host's command log.
    pub(super) fn net_reconnect(&mut self, net: &mut NetSession) -> bool {
        let Some(backoff) = net.reconnect.as_mut() else {
            return true;
        };
        let now = Instant::now();
        if !backoff.due(now) {
            return true;
        }
        let local = net.lock.local_peer;
        match galaxycraft::net::request_resync(
            &net.host_addr,
            net.map_hash,
            galaxycraft::net::WORLD_SEED,
            local,
        ) {
            Ok((mut t, turn, rec)) => {
                if !self.build_from_save(&net.save) {
                    return false;
                }
                let _ = t.stream_timeout(NET_READ_TIMEOUT);
                let mut lock = Lockstep::new(local, net.lock.peers, net.lock.ticks_per_turn);
                // `build_from_save` made fresh computers for a lobby game.
                if net.save.setup.is_some() {
                    lock.computers = std::mem::take(&mut self.ais);
                }
                galaxycraft::net::apply_resync(&mut self.world, &mut lock, turn, &rec);
                net.lock = lock;
                net.peers = vec![t];
                net.reconnect = None;
                self.accumulator = 0.0;
                self.last_step = Instant::now();
                self.status(format!("reconnected at turn {turn}"));
                true
            }
            Err(e) => {
                if backoff.fail(Instant::now()) {
                    self.status(format!(
                        "host unreachable ({e}) - retry {} of 8",
                        backoff.failures() + 1
                    ));
                    true
                } else {
                    self.status(format!("LAN disconnected: {e}"));
                    false
                }
            }
        }
    }

    /// One lockstep turn per accumulated `ticks_per_turn`.
    pub(super) fn update_net(&mut self) {
        let Some(mut net) = self.net.take() else {
            return;
        };
        let chat = net.lock.take_chat();
        let local_peer = net.lock.local_peer;
        self.push_net_chat(chat, local_peer);
        if self.game_result.is_some() {
            self.net = Some(net);
            return;
        }
        // A broken link pauses the match until it is repaired.
        if net.waiting.is_some() || net.reconnect.is_some() {
            let alive = match net.role {
                NetRole::Host => self.net_wait_for_rejoin(&mut net),
                NetRole::Client => self.net_reconnect(&mut net),
            };
            if alive {
                self.net = Some(net);
            }
            return;
        }
        let now = Instant::now();
        let dt = now.duration_since(self.last_step).as_secs_f32().min(0.1);
        self.last_step = now;
        self.accumulator += dt * TICKS_PER_REAL_SECOND * self.speed;
        let need = net.lock.ticks_per_turn as f32;
        if self.accumulator < need {
            self.net = Some(net);
            return;
        }
        self.accumulator -= need;

        let acts = net.lock.take_local();
        let mut dropped_peer = None;
        let res = match net.role {
            NetRole::Host => galaxycraft::net::host_turn_peers(
                &mut net.lock,
                &mut self.world,
                &mut net.peers,
                acts,
            )
            .map_err(|f| {
                dropped_peer = Some(f.peer);
                net.waiting = Some((f.peer, Instant::now() + REJOIN_WAIT));
                format!(
                    "player {} dropped ({}) - waiting for it",
                    f.peer + 1,
                    f.error
                )
            }),
            NetRole::Client => match net.peers.first_mut() {
                Some(host) => {
                    client_turn(&mut net.lock, &mut self.world, host, acts).map_err(|e| {
                        net.reconnect = Some(galaxycraft::net::Backoff::lan(Instant::now()));
                        format!("lost the host ({e}) - reconnecting")
                    })
                }
                None => Err("no host transport".to_string()),
            },
        };
        match res {
            Ok(desync) => {
                net.desync |= desync;
                net.last_hash = state_hash(&self.world);
                if desync {
                    self.status("LAN DESYNC - match halted".to_string());
                    self.paused = true;
                }
            }
            Err(e) => {
                self.status(e);
                if net.waiting.is_none() && net.reconnect.is_none() {
                    return;
                }
            }
        }
        let chat = net.lock.take_chat();
        let local_peer = net.lock.local_peer;
        self.push_net_chat(chat, local_peer);
        self.selected
            .retain(|id| self.world.unit(*id).is_some_and(|u| u.alive));
        self.net = Some(net);
        self.check_game_result();
    }

    /// Route an order: queued for the next network turn, or applied locally.
    pub(super) fn send_order(&mut self, unit: u32, order: Order) {
        match self.net.as_mut() {
            Some(net) => net.lock.queue(RecAct::Issue { unit, order }),
            None => {
                let _ = self.world.issue(unit, order);
            }
        }
    }

    /// Shift/LT right-click: append the order instead of replacing the
    /// current one.  Goes through the recorded `QueueOrder` action so saves,
    /// replays and LAN lockstep stay deterministic.
    pub(super) fn send_order_or_queue(&mut self, unit: u32, order: Order, queue: bool) {
        if !queue {
            self.send_order(unit, order);
            return;
        }
        match self.net.as_mut() {
            Some(net) => net.lock.queue(RecAct::QueueOrder { unit, order }),
            None => {
                let _ = self.world.queue_order_recorded(unit, order);
            }
        }
    }

    /// Add one chat line, keeping the newest 50.  The mature-language
    /// filter masks whole words in both single-player prompts and received
    /// LAN lines.
    pub(super) fn push_chat_line(&mut self, line: String) {
        let line = if self.game_options.game_language_filter {
            mature_language_filter(&line)
        } else {
            line
        };
        self.chat_log.push_back(line);
        while self.chat_log.len() > 50 {
            self.chat_log.pop_front();
        }
    }

    /// Move the chat lines read during a network turn into the on-screen log.
    pub(super) fn push_net_chat(&mut self, chat: Vec<(u8, String)>, local_peer: u8) {
        for (peer, text) in chat {
            let name = if peer == local_peer {
                self.active_name()
            } else {
                format!("Player {}", peer + 1)
            };
            self.push_chat_line(format!("{name}: {text}"));
        }
    }

    /// Recognise one of the retail single-player cheat codes.  The code is
    /// a recorded [`RecAct::Cheat`], so its resource/upgrade/toggle effects
    /// survive a save or replay.
    pub(super) fn apply_cheat_code(&mut self, text: &str) -> bool {
        use galaxycraft::sim::cheat;
        let code = text.trim().to_ascii_lowercase();
        let id = match code.as_str() {
            "show me the money" => cheat::SHOW_ME_THE_MONEY,
            "whats mine is mine" | "what's mine is mine" => cheat::WHATS_MINE_IS_MINE,
            "breathe deep" | "breathe deeply" => cheat::BREATHE_DEEP,
            "power overwhelming" => cheat::POWER_OVERWHELMING,
            "the gathering" => cheat::THE_GATHERING,
            "food for thought" => cheat::FOOD_FOR_THOUGHT,
            "black sheep wall" => cheat::BLACK_SHEEP_WALL,
            "war aint what it used to be" | "war ain't what it used to be" => {
                cheat::WAR_AINT_WHAT_IT_USED_TO_BE
            }
            "operation cwal" => cheat::OPERATION_CWAL,
            "something for nothing" => cheat::SOMETHING_FOR_NOTHING,
            "medieval man" => cheat::MEDIEVAL_MAN,
            "modify the phase variance" => cheat::MODIFY_THE_PHASE_VARIANCE,
            "there is no cow level" => cheat::THERE_IS_NO_COW_LEVEL,
            "game over man" => cheat::GAME_OVER_MAN,
            "staying alive" => cheat::STAYING_ALIVE,
            "noglues" => cheat::NOGLUES,
            _ => return false,
        };
        self.queue_act(RecAct::Cheat {
            player: self.local_player as u8,
            code: id,
        });
        self.push_chat_line("Cheat enabled.".to_string());
        self.status("Cheat enabled".to_string());
        self.play("ui_click");
        true
    }

    /// Send a chat line to the other players.  Chat is a client feature: it
    /// bypasses the command log and the lockstep checksum by design.
    pub(super) fn send_chat(&mut self, text: String) {
        if self.net.is_none() {
            // Single-player has no chat peers, but the message prompt is
            // where the retail cheat codes are typed.
            if !self.apply_cheat_code(&text) && !text.trim().is_empty() {
                self.push_chat_line(format!("{}: {text}", self.active_name()));
            }
            return;
        }
        let text: String = text.chars().take(200).collect();
        let name = self.active_name();
        if let Some(net) = self.net.as_mut() {
            match net.role {
                NetRole::Host => {
                    for p in &mut net.peers {
                        let _ = p.send(&Msg::Chat {
                            from: 0,
                            text: text.clone(),
                        });
                    }
                }
                NetRole::Client => {
                    if let Some(host) = net.peers.first_mut() {
                        let _ = host.send(&Msg::Chat {
                            from: 0,
                            text: text.clone(),
                        });
                    }
                }
            }
        }
        self.push_chat_line(format!("{name}: {text}"));
    }

    /// Same routing for production/research so both peers issue identical
    /// commands (the AI and the player both go through here).
    pub(super) fn send_train(&mut self, building: u32, unit_type: u16) -> bool {
        match self.net.as_mut() {
            Some(net) => {
                net.lock.queue(RecAct::Train {
                    building,
                    unit: unit_type,
                });
                true
            }
            None => self.world.train(building, unit_type).is_ok(),
        }
    }

    /// Queue a direct command-card act for the LAN lockstep, or apply it
    /// locally.  The acts carry their own validation in `World`.
    pub(super) fn queue_act(&mut self, act: galaxycraft::replay::RecAct) {
        if let Some(net) = self.net.as_mut() {
            net.lock.queue(act);
        } else {
            galaxycraft::replay::apply_act(&mut self.world, &act);
        }
    }

    /// Whether the game is a multiplayer one for the in-game dialogs (a
    /// lobby game, LAN or against computers): Pause Game and Surrender.
    pub(super) fn multiplayer_game(&self) -> bool {
        self.mp.setup.is_some() || self.net.is_some()
    }
}
