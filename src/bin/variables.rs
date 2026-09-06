#![allow(unused)]

fn main() {
    let x: i32 = 24;
    // This will not compile
    // x += 1;

    let mut y = 150;
    y += 1;
    println!("{0}", y);
    let w = 123;

    // Get the value from the stdlib rather than redefining it
    let x = std::f64::consts::PI;
    println!("{0}", x);

    let v: Vec<u32> = vec![1, 2, 3];
}
