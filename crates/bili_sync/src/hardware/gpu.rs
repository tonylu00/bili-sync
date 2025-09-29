use base64::{Engine as _, engine::general_purpose};

#[derive(Debug, Clone)]
pub enum GpuVendor {
    Nvidia,
    Amd,
    Intel,
}

#[derive(Debug, Clone)]
pub struct GpuInfo {
    pub vendor: GpuVendor,
    pub model: String,
    pub device_id: String,
    pub driver_version: String,
    pub directx_version: String,
    pub angle_info: String,
}

impl GpuInfo {
    pub fn nvidia_rtx4070ti() -> Self {
        Self {
            vendor: GpuVendor::Nvidia,
            model: "NVIDIA GeForce RTX 4070 Ti SUPER".to_string(),
            device_id: "0x00002705".to_string(),
            driver_version: "vs_5_0 ps_5_0".to_string(),
            directx_version: "Direct3D11".to_string(),
            angle_info: "ANGLE (NVIDIA, NVIDIA GeForce RTX 4070 Ti SUPER (0x00002705) Direct3D11 vs_5_0 ps_5_0, D3D11)Google Inc. (NVIDIA)".to_string(),
        }
    }

    pub fn nvidia_rtx4080() -> Self {
        Self {
            vendor: GpuVendor::Nvidia,
            model: "NVIDIA GeForce RTX 4080".to_string(),
            device_id: "0x00002782".to_string(),
            driver_version: "vs_5_0 ps_5_0".to_string(),
            directx_version: "Direct3D11".to_string(),
            angle_info: "ANGLE (NVIDIA, NVIDIA GeForce RTX 4080 (0x00002782) Direct3D11 vs_5_0 ps_5_0, D3D11)Google Inc. (NVIDIA)".to_string(),
        }
    }

    pub fn amd_rx7800xt() -> Self {
        Self {
            vendor: GpuVendor::Amd,
            model: "AMD Radeon RX 7800 XT".to_string(),
            device_id: "0x0000747E".to_string(),
            driver_version: "vs_5_0 ps_5_0".to_string(),
            directx_version: "Direct3D11".to_string(),
            angle_info: "ANGLE (AMD, AMD Radeon RX 7800 XT (0x0000747E) Direct3D11 vs_5_0 ps_5_0, D3D11)ATI Technologies Inc.".to_string(),
        }
    }

    pub fn intel_arc_a770() -> Self {
        Self {
            vendor: GpuVendor::Intel,
            model: "Intel Arc A770 Graphics".to_string(),
            device_id: "0x000056A0".to_string(),
            driver_version: "vs_5_0 ps_5_0".to_string(),
            directx_version: "Direct3D11".to_string(),
            angle_info: "ANGLE (Intel, Intel Arc A770 Graphics (0x000056A0) Direct3D11 vs_5_0 ps_5_0, D3D11)Intel Inc.".to_string(),
        }
    }

    pub fn to_dm_cover_img_str(&self) -> String {
        general_purpose::STANDARD.encode(&self.angle_info)
    }

    pub fn get_vendor_name(&self) -> &'static str {
        match self.vendor {
            GpuVendor::Nvidia => "NVIDIA",
            GpuVendor::Amd => "AMD",
            GpuVendor::Intel => "Intel",
        }
    }

    pub fn get_full_info(&self) -> String {
        format!("{} {} {}", self.get_vendor_name(), self.model, self.device_id)
    }
}