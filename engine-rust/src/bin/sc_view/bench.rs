//! `GC_BENCH=N`: run N frames of the live loop (update + draw) on the
//! captured screen and report the frame cost (in-game performance).
//!
//! `GC_BENCH_SCROLL=1` holds the pointer at a screen edge (the direction
//! reversed every 120 frames, 60 Hz frame time) so every frame scrolls,
//! `GC_BENCH_SCROLL=pad` the controller's right stick instead;
//! `GC_BENCH_SLOW=1` lists the frames over 16.7 ms.

use super::*;

pub(super) fn run(app: &mut App, fb: &mut Framebuffer, screen: &str, n: u32) {
    // Measured as a live game: portrait clips decode in the background.
    app.headless = false;
    let scroll = std::env::var_os("GC_BENCH_SCROLL").is_some();
    // `GC_BENCH_SCROLL=pad`: the right stick held on the diagonal instead.
    let pad = std::env::var("GC_BENCH_SCROLL").is_ok_and(|v| v == "pad");
    let list_slow = std::env::var_os("GC_BENCH_SLOW").is_some();
    let (mut upd, mut drw, mut worst, mut worst_warm) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
    let (cam0, mut moved, mut slow) = (app.camera, 0i64, 0u32);
    for f in 0..n {
        if scroll {
            app.edge_scroll = (!pad).then_some(if (f / 120) % 2 == 0 { 3 } else { 7 });
            app.frame_dt = Some(1.0 / 60.0);
        }
        let before = app.camera;
        let t0 = Instant::now();
        if scroll {
            let sign = if (f / 120) % 2 == 0 { 1 } else { -1 };
            let p = gamepad::PAN_SPEED * sign;
            let input = GameInput {
                pan: if pad { (p, p) } else { (0, 0) },
                ..GameInput::default()
            };
            app.handle_game_input(&input);
        }
        app.update();
        let t1 = Instant::now();
        app.draw(fb);
        let t2 = Instant::now();
        upd += (t1 - t0).as_secs_f64() * 1000.0;
        drw += (t2 - t1).as_secs_f64() * 1000.0;
        let ms = (t2 - t0).as_secs_f64() * 1000.0;
        worst = worst.max(ms);
        if f >= 10 {
            worst_warm = worst_warm.max(ms);
        }
        if ms > 16.7 {
            slow += 1;
            if list_slow {
                let u = (t1 - t0).as_secs_f64() * 1000.0;
                eprintln!(
                    "slow frame {f}: {ms:.1} ms (update {u:.1}) camera {:?} tick {}",
                    app.camera, app.world.tick
                );
            }
        }
        moved += ((app.camera.0 - before.0).abs() + (app.camera.1 - before.1).abs()) as i64;
    }
    if scroll {
        eprintln!(
            "bench scroll: camera {cam0:?} -> {:?}, {moved} px moved",
            app.camera
        );
    }
    eprintln!("bench after 10 frames: worst {worst_warm:.2} ms; {slow} frames over 16.7 ms");
    eprintln!(
        "bench {screen} {}x{}: {n} frames, update {:.2} ms, draw {:.2} ms, worst {:.2} ms, tick {}",
        fb.width,
        fb.height,
        upd / n.max(1) as f64,
        drw / n.max(1) as f64,
        worst,
        app.world.tick
    );
}

/// `GC_FRAME_LOG=path`: the live window loop appends one line per frame
/// (`ms camera_x camera_y edge`), to measure real scrolling.
pub(super) fn log_frame(app: &App) {
    thread_local! {
        static LOG: std::cell::RefCell<Option<std::fs::File>> = std::cell::RefCell::new(
            std::env::var_os("GC_FRAME_LOG").and_then(|p| std::fs::File::create(p).ok()),
        );
    }
    LOG.with(|l| {
        if let (Some(f), Some(dt)) = (l.borrow_mut().as_mut(), app.frame_dt) {
            use std::io::Write;
            let _ = writeln!(
                f,
                "{:.2} {} {} {:?} {}",
                dt * 1000.0,
                app.camera.0,
                app.camera.1,
                app.edge_scroll,
                app.menu
            );
        }
    });
}
