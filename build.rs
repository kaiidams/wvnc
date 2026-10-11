fn main() {
    cc::Build::new()
        .file("src-c/d3des.c")
        .compile("d3des");
}
