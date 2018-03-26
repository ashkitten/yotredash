extern crate gcc;

fn main() {
    gcc::Build::new()
        .file("library_messages.c")
        .compile("library_messages");
    println!("cargo:rustc-link-lib=jack");
}
