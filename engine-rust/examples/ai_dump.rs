//! Print a campaign AI script's instructions: `cargo run --example ai_dump -- Ter3`.
use galaxycraft::data::GameData;

fn main() {
    let d = GameData::bundled().unwrap();
    for id in std::env::args().skip(1) {
        let mut key = [0u8; 4];
        for (i, b) in id.bytes().take(4).enumerate() {
            key[i] = b;
        }
        let Some(s) = d.campaign_ai_script(&key) else {
            println!("{id}: missing");
            continue;
        };
        for ins in s.instructions() {
            println!("{:5} {:<24} {:?}", ins.offset, ins.name(), ins.args);
        }
    }
}
