pub mod gt911;
pub mod lcd_panel;

use log::*;

use cstr_core::CString;

use anyhow::Error;

use std::cell::RefCell;
use std::time::Instant;

use esp_idf_hal::{
    delay::{Ets, FreeRtos},
    gpio::PinDriver,
    i2c::{I2cConfig, I2cDriver},
    peripherals::Peripherals,
    units::FromValueType,
};

use esp_idf_hal::ledc::{
    config::TimerConfig,
    {LedcDriver, LedcTimerDriver},
};

use lvgl::style::Style;
use lvgl::widgets::{Arc, Label};
use lvgl::{Align, Color, Display, DrawBuffer, Part, Widget};

use embedded_graphics_core::prelude::Point;
use lvgl::input_device::{
    pointer::{Pointer, PointerInputData},
    InputDriver,
};

use crate::gt911::GT911;
use crate::lcd_panel::{LcdPanel, PanelConfig, PanelFlagsConfig, TimingFlagsConfig, TimingsConfig};

fn mem_info() -> lvgl_sys::lv_mem_monitor_t {
    let mut info = lvgl_sys::lv_mem_monitor_t {
        total_size: 0,
        free_cnt: 0,
        free_size: 0,
        free_biggest_size: 0,
        used_cnt: 0,
        max_used: 0,
        used_pct: 0,
        frag_pct: 0,
    };
    unsafe {
        lvgl_sys::lv_mem_monitor(&mut info as *mut _);
    }
    info
}

fn main() -> anyhow::Result<(), anyhow::Error> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    log::info!("================ Staring App ================");

    const HOR_RES: u32 = 800;
    const VER_RES: u32 = 480;
    const LINES: u32 = 12; // The number of lines (rows) that will be refreshed

    let peripherals = Peripherals::take()?;

    #[allow(unused)]
    let pins = peripherals.pins;

    //============================================================================================================
    //               Create the I2C to communicate with the touchscreen controller
    //============================================================================================================
    let i2c = peripherals.i2c0;
    let sda = pins.gpio19;
    let scl = pins.gpio20;
    let config = I2cConfig::new().baudrate(100.kHz().into());
    let i2c = I2cDriver::new(i2c, sda, scl, &config)?;
    let rst = PinDriver::output(pins.gpio38)?; // reset pin on GT911

    //============================================================================================================
    //               Create the LedcDriver to drive the backlight on the Lcd Panel
    //============================================================================================================
    let mut channel = LedcDriver::new(
        peripherals.ledc.channel0,
        LedcTimerDriver::new(
            peripherals.ledc.timer0,
            &TimerConfig::new().frequency(25.kHz().into()),
        )
        .unwrap(),
        pins.gpio2,
    )?;
    channel.set_duty(channel.get_max_duty() / 2)?;
    info!("============= Backlight turned on =============");

    // Initialize lvgl
    lvgl::init();
    info!("meminfo init: {:?}", mem_info());

    //=====================================================================================================
    //                         Create the LCD Display
    //=====================================================================================================
    let mut lcd_panel = LcdPanel::new(
        &PanelConfig::new(),
        &PanelFlagsConfig::new(),
        &TimingsConfig::new(),
        &TimingFlagsConfig::new(),
    )?;

    info!("=============  Registering Display ====================");
    let buffer = DrawBuffer::<{ (HOR_RES * LINES) as usize }>::default();
    let display = Display::register(buffer, HOR_RES, VER_RES, |refresh| {
        lcd_panel
            .set_pixels_lvgl_color(
                refresh.area.x1.into(),
                refresh.area.y1.into(),
                (refresh.area.x2 + 1i16).into(),
                (refresh.area.y2 + 1i16).into(),
                refresh.colors.into_iter(),
            )
            .unwrap();
    })
    .map_err(Error::msg)?;

    //======================================================================================================
    //                          Create the driver for the Touchscreen
    //======================================================================================================
    let gt911_touchscreen = RefCell::new(GT911::new(i2c, rst, Ets));
    gt911_touchscreen.borrow_mut().reset()?;

    // The read_touchscreen_cb is used by Lvgl to detect touchscreen presses and releases
    let read_touchscreen_cb = || {
        let touch = gt911_touchscreen.borrow_mut().read_touch().unwrap();

        match touch {
            Some(tp) => PointerInputData::Touch(Point::new(tp.x as i32, tp.y as i32))
                .pressed()
                .once(),
            None => PointerInputData::Touch(Point::new(0, 0)).released().once(),
        }
    };

    info!("=============  Registering Touchscreen ====================");
    let _touch_screen = Pointer::register(read_touchscreen_cb, &display).map_err(Error::msg)?;

    //=======================================================================================================
    //                               Create the User Interface
    //=======================================================================================================
    // Create screen and widgets
    let mut screen = display.get_scr_act().map_err(Error::msg)?;
    let mut screen_style = Style::default();
    screen_style.set_bg_color(Color::from_rgb((0, 0, 0)));
    screen_style.set_radius(0);
    screen.add_style(Part::Main, &mut screen_style);

    // Create the arc object
    let mut arc = Arc::create(&mut screen).map_err(Error::msg)?;
    arc.set_size(150, 150);
    arc.set_align(Align::Center, 0, 0);
    arc.set_start_angle(135).map_err(Error::msg)?;
    arc.set_end_angle(135).map_err(Error::msg)?;

    // Create loading label
    let mut loading_lbl = Label::create(&mut screen).map_err(Error::msg)?;
    loading_lbl
        .set_text(CString::new("Loading...").unwrap().as_c_str())
        .map_err(Error::msg)?;
    loading_lbl.set_align(Align::Center, 0, 0);

    // Create style for the loading label
    let mut loading_style = Style::default();
    loading_style.set_text_color(Color::from_rgb((0, 0, 255)));
    loading_lbl.add_style(Part::Main, &mut loading_style);

    let mut angle = 0;
    let mut forward = true;
    let mut i = 0;

    loop {
        let start = Instant::now();

        if i > 270 {
            forward = !forward;
            i = 1;
            info!("meminfo running: {:?}", mem_info());
        }
        angle = if forward { angle + 1 } else { angle - 1 };
        arc.set_end_angle(angle + 135).map_err(Error::msg)?;
        i += 1;

        lvgl::task_handler();

        // Keep the loop delay short so Lvgl can respond quickly to touchscreen presses and releases
        FreeRtos::delay_ms(20);

        lvgl::tick_inc(Instant::now().duration_since(start));
    }
}
