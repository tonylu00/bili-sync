use super::{HardwareInfo, GpuVendor};
use rand::Rng;
use serde_json::json;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct HardwareFingerprint {
    hardware: HardwareInfo,
    screen_resolution: (u32, u32),
    device_pixel_ratio: f32,
    timezone_offset: i32,
}

impl HardwareFingerprint {
    pub fn new(hardware: HardwareInfo) -> Self {
        Self {
            hardware,
            screen_resolution: (2560, 1440), // 常见的2K分辨率
            device_pixel_ratio: 1.0,
            timezone_offset: -480, // 中国时区 UTC+8
        }
    }

    pub fn generate_dm_img_list(&self, video_duration: u32) -> String {
        let mut rng = rand::thread_rng();
        let mut interactions = Vec::new();

        // 生成合理的用户交互数据
        let interaction_count = rng.gen_range(3..=6);
        for i in 0..interaction_count {
            let timestamp = rng.gen_range(0..std::cmp::min(video_duration * 1000, 30000)); // 最多30秒
            let x = rng.gen_range(1000..self.screen_resolution.0);
            let y = rng.gen_range(500..self.screen_resolution.1);
            let z = if i == 0 { 0 } else { rng.gen_range(50..200) };
            let k = rng.gen_range(80..100);

            interactions.push(json!({
                "x": x,
                "y": y,
                "z": z,
                "timestamp": timestamp,
                "k": k,
                "type": 0
            }));
        }

        serde_json::to_string(&interactions).unwrap_or_else(|_| "[]".to_string())
    }

    pub fn generate_dm_img_inter(&self) -> String {
        let mut rng = rand::thread_rng();

        let ds_data = json!([{
            "t": 2,
            "c": "dmlkZW8tY29udGFpbmVyLX", // base64编码的"video-container-"
            "p": [
                rng.gen_range(200..300),
                rng.gen_range(30..50),
                rng.gen_range(280..300)
            ],
            "s": [
                rng.gen_range(400..600),
                rng.gen_range(18000..20000),
                rng.gen_range(-25000..-20000)
            ]
        }]);

        let inter_data = json!({
            "ds": ds_data,
            "wh": [
                self.screen_resolution.0,
                self.screen_resolution.1,
                rng.gen_range(100..120)
            ],
            "of": [
                rng.gen_range(500..520),
                rng.gen_range(1000..1020),
                rng.gen_range(500..520)
            ]
        });

        serde_json::to_string(&inter_data).unwrap_or_else(|_| "{}".to_string())
    }

    pub fn get_hardware(&self) -> &HardwareInfo {
        &self.hardware
    }

    pub fn get_screen_info(&self) -> (u32, u32, f32) {
        (self.screen_resolution.0, self.screen_resolution.1, self.device_pixel_ratio)
    }

    // 预设配置方法
    pub fn gaming_setup() -> Self {
        Self::new(HardwareInfo::nvidia_rtx4070ti())
    }

    pub fn workstation_setup() -> Self {
        Self::new(HardwareInfo::amd_rx7800xt())
    }

    pub fn budget_setup() -> Self {
        Self::new(HardwareInfo::intel_arc_a770())
    }

    // 随机选择一个常见的硬件配置
    pub fn random_common_setup() -> Self {
        let mut rng = rand::thread_rng();
        match rng.gen_range(0..3) {
            0 => Self::gaming_setup(),
            1 => Self::workstation_setup(),
            _ => Self::budget_setup(),
        }
    }
}

impl Default for HardwareFingerprint {
    fn default() -> Self {
        Self::gaming_setup()
    }
}