use ispc::*;

fn main() {
    Config::new()
        .opt_level(3)
        .file("src/test.ispc")
        .compile("test");
}
