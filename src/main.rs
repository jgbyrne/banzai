mod bwt;

fn main() {
    let test = "americanfootball";
    let (bwt, start) = bwt::bwt(String::from(test).into_bytes());
    let bwt = String::from_utf8(bwt).unwrap();
    println!("{} {}", bwt, start);
}
