// Entry point. The same binary has two modes:
//   soplos-linux-installer           the window, run as the live user
//   soplos-linux-installer --helper  the installation itself, run as root
//                                    through `sudo -n` (DESIGN.md, section 2)

fn main() {
    if std::env::args().nth(1).as_deref() == Some("--helper") {
        std::process::exit(soplos_linux_installer_lib::helper::run());
    }

    // WebKitGTK's DMA-BUF renderer causes protocol errors (Error 71) on Wayland
    // with NVIDIA/Nouveau drivers on physical hardware, as well as blank windows
    // on virtual GPUs. Force it disabled by default on all hardware.
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }

    soplos_linux_installer_lib::run();
}
