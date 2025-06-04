pub mod net;
pub mod io;
pub mod rand;
pub mod graphics;
pub mod ui;
pub mod primitives;
pub mod collections;


use std::time::Instant;

use collections::FixedVec;

const SIZE: usize = 100000000;


fn main() {
    
        let start_time = Instant::now();
        let mut vec: FixedVec<u16> = FixedVec::with_value(SIZE, 2);
        let vec2 = FixedVec::with_value(vec.len(), 1);
        
        for i in 0..vec.len() {
            vec[i] += vec2[i];
        }
        
        let end_time = start_time.elapsed();
    
        println!("time manual: {:?}, {}", end_time, vec[0]);
    let start_time = Instant::now();
    let mut vec: FixedVec<u16> = FixedVec::with_value(SIZE, 2);
    let vec2: FixedVec<u16> = FixedVec::with_value(vec.len(), 1);

    vec += vec2;
    let end_time = start_time.elapsed();



    println!("time trait: {:?}, {}", end_time, vec[0]);

}

#[inline(always)]
#[allow(unused)]
    fn add_assign(vec: &mut FixedVec<u16>, vec2: &FixedVec<u16>) {
        //debug_assert!(this.len() == rhs.len(), "Length mismatch");

        for i in 0..vec.len() {
            vec[i] += vec2[i];
        }
    }