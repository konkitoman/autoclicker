use clap::Parser;

#[derive(clap::Subcommand, Debug)]
pub enum Command {
    Run {
        /// Device name or path when the first character is `/`
        /// (first looks for exact match, then takes the first device that contains the name)
        #[arg(short = 'd')]
        device_query: String,

        /// Bind left autoclicker to keycode
        /// Mouse: 275 ButtonSide
        /// Keyboard: 26 LeftBrace
        #[arg(short)]
        left_bind: Option<u16>,

        /// Bind middle autoclicker to keycode
        /// Mouse: 274 ButtonMiddle
        /// Keyboard: 43 Backslash
        #[arg(short)]
        middle_bind: Option<u16>,

        /// Bind right autoclicker to keycode
        /// Mouse: 276 ButtonExtra
        /// Keyboard: 27 RightBrace
        #[arg(short)]
        right_bind: Option<u16>,

        /// Bind lock/unlock to keycode
        /// Mouse: 274 ButtonMiddle
        /// With this you can bind to the left, middle and right button, and the bindings will only be used when is unlocked.
        /// Useful for mouses without side buttons.
        #[arg(short = 'T')]
        lock_unlock_bind: Option<u16>,

        /// Hold mode, when a keybind is pressed the autoclicker will be active until the keybind release
        #[arg(short = 'H', default_value_t = false)]
        hold: bool,

        /// This will grab the device,
        #[arg(long, default_value_t = false)]
        grab: bool,

        /// Set the cooldown in milliseconds
        #[arg(short, default_value_t = 25)]
        cooldown: u64,

        /// Set cooldown in milliseconds, between press and release
        #[arg(short = 'C', default_value_t = 0)]
        cooldown_press_release: u64,
    },
    RunLegacy {
        /// Device name or path when the first character is `/`
        /// (first looks for exact match, then takes the first device that contains the name)
        #[arg(short = 'd')]
        device_query: String,

        /// Set the cooldown in milliseconds
        #[arg(short, default_value_t = 25)]
        cooldown: u64,

        /// Set cooldown in milliseconds, between press and release
        #[arg(short = 'C', default_value_t = 0)]
        cooldown_press_release: u64,
    },
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[arg(long, default_value_t = false)]
    pub debug: bool,

    /// For not beeping when the autoclicker state is changed
    #[arg(long, default_value_t = false)]
    pub beep: bool,

    #[command(subcommand)]
    pub command: Option<Command>,
}
