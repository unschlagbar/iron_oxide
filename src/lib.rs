pub mod collections;
#[cfg(feature = "graphics")]
pub mod graphics;
pub mod io;
pub mod net;
pub mod physics;
pub mod physics2d;
pub mod primitives;
pub mod rand;
pub mod security;
#[cfg(feature = "graphics")]
pub mod ui;

#[cfg(test)]
mod tests {

    #[test]
    fn it_works() {}
}
