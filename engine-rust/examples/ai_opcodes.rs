//! Print the AI opcodes each campaign script uses (scripting aid).
use galaxycraft::data::GameData;
fn main() {
    let d = GameData::bundled().unwrap();
    let implemented: Vec<u8> = std::env::args()
        .nth(1)
        .unwrap_or_default()
        .split(',')
        .filter_map(|s| s.parse().ok())
        .collect();
    for id in std::env::args().skip(2) {
        let mut key = [0u8; 4];
        for (i, b) in id.bytes().take(4).enumerate() {
            key[i] = b;
        }
        let Some(s) = d.campaign_ai_script(&key) else {
            println!("{id}: missing");
            continue;
        };
        let mut ops: Vec<u8> = s.instructions().iter().map(|i| i.opcode).collect();
        ops.sort();
        ops.dedup();
        let missing: Vec<u8> = ops
            .iter()
            .copied()
            .filter(|o| !implemented.contains(o))
            .collect();
        println!(
            "{id}: {} instrs, missing opcodes {:?}",
            s.instructions().len(),
            missing
        );
    }
}
