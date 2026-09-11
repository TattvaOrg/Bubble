fn main() {
    println!("cargo:rustc-link-lib=crypto");
    println!("cargo:rustc-link-lib=argon2");
    println!("cargo:rustc-link-lib=sqlite3");
}
