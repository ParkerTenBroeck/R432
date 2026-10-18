pub mod reg;
pub mod term;

unsafe extern "C" {
    #[link_name = "random_source"]
    pub safe static random_source: reg::R32;

    #[link_name = "frame_clock"]
    pub safe static frame_clock: reg::R32;

    #[link_name = "terminal"]
    pub safe static terminal: term::Terminal;
}
