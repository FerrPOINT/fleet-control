#[allow(dead_code)]
mod original {
    include!(concat!(env!("OUT_DIR"), "/original.rs"));
    include!("scenario.rs");
}

fn main() {
    original::cut_entry();
}
