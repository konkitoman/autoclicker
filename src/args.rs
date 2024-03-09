use clap::Parser;

#[derive(Parser, Debug, serde::Serialize, serde::Deserialize)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Cleaning the cache
    #[arg(long, default_value_t = false)]
    pub clear_cache: bool,

    /// Set the cooldown in milliseconds
    #[arg(short, long, default_value_t = 25)]
    pub cooldown: u64,

    /// Set cooldown in milliseconds, between press and release
    #[arg(short = 'C', long, default_value_t = 0)]
    pub cooldown_press_release: u64,

    /// Bind left autoclicker to keycode
    #[arg(short, long)]
    pub left_bind: Option<u16>,

    /// Bind right autoclicker to keycode
    #[arg(short, long)]
    pub right_bind: Option<u16>,

    /// For finding what key is pressed
    #[arg(short, long, default_value_t = false)]
    pub find_keycodes: bool,

    /// For not beeping when the autoclicker state is changed
    #[arg(short, long, default_value_t = false)]
    pub no_beep: bool,

    #[arg(long, default_value_t = false)]
    pub debug: bool,

    /// I will not: grab the input device and emulates any unused action from it
    #[arg(long, default_value_t = false)]
    pub no_grab: bool,

    /// This will let the keyboard to be grabbed!
    #[arg(long, default_value_t = false)]
    pub grab_kbd: bool,

    /// Automatically uses the specified device by name
    /// (first looks for exact match, then takes the first device that contains the string)
    #[arg(short = 'd', long)]
    pub use_device: Option<String>,

    /// This will try to open the specified dev path, you need to specify what type of device is!
    #[arg(short = 'D')]
    pub use_dev_path: Option<String>,

    /// Set the device type as Keyboard
    #[arg(short = 'K', default_value_t = false)]
    pub is_keyboard: bool,

    /// Set the device type as Mouse
    #[arg(short = 'M', default_value_t = false)]
    pub is_mouse: bool,
}
