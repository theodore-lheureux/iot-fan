use std::sync::{Arc, Mutex};

use anyhow::Result;
use embedded_svc::{
    http::Method,
    io::Write,
};
use esp_idf_hal::{gpio::PinDriver, prelude::Peripherals};
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    http::server::{Configuration, EspHttpServer},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use wifi::wifi;

// If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use esp_idf_sys::{self};

use iot_fan::{
    self,
    fan::{Fan, Speed},
    leds::LEDs,
};
use log::info;

/// This configuration is picked up at compile time by `build.rs` from the
/// file `cfg.toml`.
#[toml_cfg::toml_config]
pub struct Config {
    #[default("")]
    wifi_ssid: &'static str,
    #[default("")]
    wifi_psk: &'static str,
    #[default("12345")]
    id: &'static str,
    #[default("action.devices.types.FAN")]
    r#type: &'static str,
    #[default(&[
        "action.devices.traits.OnOff",
        "action.devices.traits.FanSpeed",
    ])]
    traits: &'static [&'static str],
    #[default("Fan")]
    name: &'static str,
    #[default(false)]
    will_report_state: bool,
}

fn main() -> Result<()> {
    esp_idf_sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take().unwrap();
    let sysloop = EspSystemEventLoop::take()?;

    // The constant `CONFIG` is auto-generated by `toml_config`.
    let app_config = CONFIG;
    let mut wifi_indicator_led = PinDriver::output(peripherals.pins.gpio2)?;

    // Connect to the Wi-Fi network
    let _wifi = match wifi(
        app_config.wifi_ssid,
        app_config.wifi_psk,
        peripherals.modem,
        sysloop,
    ) {
        Ok(inner) => {
            wifi_indicator_led.set_high()?;
            inner
        },
        Err(err) => {
            info!("Could not connect to Wi-Fi network: {:?}", err);
            loop {
                wifi_indicator_led.toggle()?;
                std::thread::sleep(std::time::Duration::from_millis(500));
            }
        }
    };

    let mut leds = LEDs::new(
        peripherals.pins.gpio32,
        peripherals.pins.gpio33,
        peripherals.pins.gpio25,
        peripherals.pins.gpio26,
    )?;
    let fan = Fan::new();

    leds.update_status(&fan)?;

    let fan = Arc::new(Mutex::new(fan));

    let mut server = EspHttpServer::new(&Configuration::default())?;

    server.fn_handler("/", Method::Get, move |req| {
        req.into_ok_response()?;
        Ok(())
    })?;

    server.fn_handler("/sync", Method::Get, move |req| {
        let mut req = req.into_response(200, None, &[("Content-Type", "application/json")])?;
        req.write_all(sync_res()?.as_bytes())?;
        Ok(())
    })?;

    let fan_clone = fan.clone();
    server.fn_handler("/query", Method::Get, move |req| {
        let fan = fan_clone.lock().unwrap();
        let mut req = req.into_response(200, None, &[("Content-Type", "application/json")])?;
        req.write_all(query_res(&fan)?.as_bytes())?;
        Ok(())
    })?;

    let fan_clone = fan.clone();
    server.fn_handler("/execute", Method::Post, move |mut req| {

        // read the request body
        let mut buf = [0u8; 1024];
        let mut body = Vec::new();
        loop {
            let len = req.read(&mut buf)?;
            if len == 0 {
                break;
            }
            body.extend_from_slice(&buf[..len]);
        }

        let body: ExecuteRequest = serde_json::from_slice(&body)?;
        let mut fan = fan_clone.lock().unwrap();

        match &*body.command {
            "action.devices.commands.OnOff" => {
                if let Some(on) = body.params.on {
                    fan.set_on(on)
                }
            }
            "action.devices.commands.SetFanSpeed" => {
                if let Some(speed) = body.params.fan_speed {
                    fan.set_speed(match &*speed {
                        "low_key" => Speed::Low,
                        "med_key" => Speed::Medium,
                        "high_key" => Speed::High,
                        _ => Speed::Low,
                    });
                }
            }
            _ => info!("Unknown command: {:?}", body),
        }

        let mut req = req.into_response(200, None, &[("Content-Type", "application/json")])?;
        req.write_all(query_res(&fan)?.as_bytes())?;
        Ok(())
    })?;

    let button = PinDriver::input(peripherals.pins.gpio4)?;
    let mut button_released = false;

    loop {
        std::thread::sleep(std::time::Duration::from_millis(10));

        let mut fan = fan.lock().unwrap();

        if button.is_high() && button_released {
            button_released = false;
            button_pressed(&mut fan)?;
        } else if button.is_low() {
            button_released = true;
        }
        leds.update_status(&fan)?;
    }
}

fn button_pressed(fan: &mut Fan) -> Result<()> {
    info!("Button pressed");
    fan.next_speed();
    Ok(())
}

fn sync_res() -> Result<String> {
    let json = json!({
        "id": CONFIG.id,
        "type": CONFIG.r#type,
        "traits": CONFIG.traits,
        "name": {
            "name": CONFIG.name
        },
        "willReportState": CONFIG.will_report_state,
        "attributes": {
            "availableFanSpeeds": {
                "speeds": [
                    {
                        "speed_name": "low_key",
                        "speed_values": [
                            {
                                "speed_synonym": ["low", "slow"],
                                "lang": "en"
                            }
                        ]
                    },
                    {
                        "speed_name": "med_key",
                        "speed_values": [
                            {
                                "speed_synonym": ["medium", "normal"],
                                "lang": "en"
                            }
                        ]
                    },
                    {
                        "speed_name": "high_key",
                        "speed_values": [
                            {
                                "speed_synonym": ["high", "fast"],
                                "lang": "en"
                            }
                        ]
                    }
                ],
                "ordered": true
            },
            "reversible": false,
        },
        "deviceInfo": {
            "manufacturer": "BigGainAFisherman Inc.",
            "model": "Super Advanced IoT Fan",
            "hwVersion": "1.0",
            "swVersion": "1.0"
        }
    });
    Ok(json.to_string())
}

fn query_res(fan: &Fan) -> Result<String> {
    let json = json!({
        CONFIG.id: {
            "status": "SUCCESS",
            "on": fan.is_on(),
            "online": true,
            "currentFanSpeedSetting": fan.get_speed().to_string()
        }
    });
    Ok(json.to_string())
}

#[derive(Serialize, Deserialize, Debug)]
struct ExecuteRequestParams {
    #[serde(rename = "fanSpeed")]
    fan_speed: Option<String>,
    on: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug)]
struct ExecuteRequest {
    id: String,
    command: String,
    params: ExecuteRequestParams,
}
