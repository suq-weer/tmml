use system_theme::ThemeColor;

/// 将 ThemeColor 转换为带 # 号的十六进制字符串 (例如: "#4A92CB")
pub fn theme_color_to_hex(color: ThemeColor) -> String {
    let r = (color.red.clamp(0.0, 1.0) * 255.0).round() as u8;
    let g = (color.green.clamp(0.0, 1.0) * 255.0).round() as u8;
    let b = (color.blue.clamp(0.0, 1.0) * 255.0).round() as u8;

    format!("#{:02X}{:02X}{:02X}", r, g, b)
}
