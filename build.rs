use std::{env, fs, io::Cursor, path::{Path, PathBuf}};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=assets/fastxcel.ico");

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR is set by Cargo"));
    let png_path = out_dir.join("fastxcel-icon.png");

    let ico_path = Path::new("assets/fastxcel.ico");
    let ico_bytes = fs::read(ico_path).expect("failed to read assets/fastxcel.ico");
    let image = image::load_from_memory_with_format(&ico_bytes, image::ImageFormat::Ico)
        .expect("failed to decode assets/fastxcel.ico");
    let mut png_bytes = Vec::new();
    image
        .write_to(&mut Cursor::new(&mut png_bytes), image::ImageFormat::Png)
        .expect("failed to encode runtime PNG icon");
    fs::write(&png_path, png_bytes).expect("failed to write runtime PNG icon");

    #[cfg(windows)]
    {
        let mut resource = winresource::WindowsResource::new();
        resource.set_icon(ico_path.to_string_lossy().as_ref());
        resource.compile().expect("failed to compile Windows resources");
    }
}
