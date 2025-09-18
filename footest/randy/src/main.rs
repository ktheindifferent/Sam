use std::fs::File ; 
use std::io::Read ; 
fn main() {
    let mut file = File::open("/dev/urandom").expect("cannot open /dev/urandom") ; 
    let mut buf = [0u8 ;  4] ; 
    file.read_exact(&mut buf).expect("cannot read") ; 
    let n = u32::from_ne_bytes(buf) ; 
    println!("Random number: {}", n) ; 
}
