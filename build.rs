use std::process::Command;

fn main() {
    let out = Command::new("python")
        .args(&["-c", "import sys; print(sys.version_info[1])"])
        .output()
        .expect("python version did not print");
    let minor = u8::from_str_radix(String::from_utf8_lossy(&out.stdout).trim(), 10)
        .expect("python version was not parsed");

    for i in 6..=minor {
        println!("cargo:rustc-cfg=Py_3_{}", i);
    }
    println!("cargo:rustc-cfg=Py_3");
}
