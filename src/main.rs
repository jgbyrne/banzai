mod bwt;
use std::fs;

fn main() {
    let (bwt, _start) = bwt::bwt(fs::read("/home/jgb/dl/silesia_xray.bin").unwrap());
    let cmp = fs::read("/home/jgb/dl/silesia_xray.bwt").unwrap();

    for i in 0..cmp.len() {
        assert!(cmp[i] == bwt[i]);
    }
    println!("success :-)")
}
