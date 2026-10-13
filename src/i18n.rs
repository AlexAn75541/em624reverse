use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize, Default)]
pub enum Language {
    #[default]
    English,
    Vietnamese,
}

impl Language {
    pub fn name(self) -> &'static str {
        match self {
            Language::English => "English",
            Language::Vietnamese => "Tiếng Việt",
        }
    }
}

pub struct Strings;

impl Strings {
    pub fn app_subtitle(lang: Language) -> &'static str {
        match lang {
            Language::English => "GAMING MOUSE CONFIGURATOR",
            Language::Vietnamese => "TRÌNH CẤU HÌNH CHUỘT GAMING",
        }
    }

    pub fn tab_game(lang: Language) -> &'static str {
        match lang {
            Language::English => "Game Mode",
            Language::Vietnamese => "Chế độ Game",
        }
    }

    pub fn tab_office(lang: Language) -> &'static str {
        match lang {
            Language::English => "Office Mode",
            Language::Vietnamese => "Chế độ Văn phòng",
        }
    }

    pub fn tab_advanced(lang: Language) -> &'static str {
        match lang {
            Language::English => "Advanced",
            Language::Vietnamese => "Nâng cao",
        }
    }

    pub fn tab_led(lang: Language) -> &'static str {
        match lang {
            Language::English => "LED Lighting",
            Language::Vietnamese => "Đèn LED RGB",
        }
    }

    pub fn tab_macro(lang: Language) -> &'static str {
        match lang {
            Language::English => "Macro Manager",
            Language::Vietnamese => "Quản lý Macro",
        }
    }

    pub fn tab_settings(lang: Language) -> &'static str {
        match lang {
            Language::English => "App Settings",
            Language::Vietnamese => "Cài đặt Ứng dụng",
        }
    }

    pub fn connected(lang: Language) -> &'static str {
        match lang {
            Language::English => "CONNECTED",
            Language::Vietnamese => "ĐÃ KẾT NỐI",
        }
    }

    pub fn disconnected(lang: Language) -> &'static str {
        match lang {
            Language::English => "DISCONNECTED",
            Language::Vietnamese => "CHƯA KẾT NỐI",
        }
    }

    pub fn check_connection(lang: Language) -> &'static str {
        match lang {
            Language::English => "Check Connection",
            Language::Vietnamese => "Kiểm tra kết nối",
        }
    }

    pub fn unfinished_adv(lang: Language) -> &'static str {
        match lang {
            Language::English => "• ADVANCED SETTINGS [UNFINISHED]",
            Language::Vietnamese => "• CÀI ĐẶT NÂNG CAO [CHƯA HOÀN THIỆN]",
        }
    }

    pub fn unfinished_led(lang: Language) -> &'static str {
        match lang {
            Language::English => "• LED LIGHTING [UNFINISHED]",
            Language::Vietnamese => "• ĐÈN LED RGB [CHƯA HOÀN THIỆN]",
        }
    }

    pub fn unfinished_macro(lang: Language) -> &'static str {
        match lang {
            Language::English => "• MACRO MANAGER [UNFINISHED]",
            Language::Vietnamese => "• QUẢN LÝ MACRO [CHƯA HOÀN THIỆN]",
        }
    }

    pub fn btn_read(lang: Language) -> &'static str {
        match lang {
            Language::English => "Read from Mouse",
            Language::Vietnamese => "Đọc từ chuột",
        }
    }

    pub fn btn_flash(lang: Language) -> &'static str {
        match lang {
            Language::English => "Flash to Mouse",
            Language::Vietnamese => "Nạp vào chuột",
        }
    }

    pub fn btn_mode_gaming(lang: Language) -> &'static str {
        match lang {
            Language::English => "Select Mode: [Gaming Mode]",
            Language::Vietnamese => "Chế độ: [Gaming Mode]",
        }
    }

    pub fn btn_mode_office(lang: Language) -> &'static str {
        match lang {
            Language::English => "Select Mode: [Office Mode]",
            Language::Vietnamese => "Chế độ: [Office Mode]",
        }
    }

    pub fn btn_load(lang: Language) -> &'static str {
        match lang {
            Language::English => "Load Profile",
            Language::Vietnamese => "Mở hồ sơ",
        }
    }

    pub fn btn_save(lang: Language) -> &'static str {
        match lang {
            Language::English => "Save Profile",
            Language::Vietnamese => "Lưu hồ sơ",
        }
    }

    pub fn game_mode_title(lang: Language) -> &'static str {
        match lang {
            Language::English => "Gaming Mode Bindings",
            Language::Vietnamese => "Gán phím Chế độ Game",
        }
    }

    pub fn office_mode_title(lang: Language) -> &'static str {
        match lang {
            Language::English => "Office Mode Bindings",
            Language::Vietnamese => "Gán phím Chế độ Văn phòng",
        }
    }

    pub fn select_key_hint(lang: Language) -> &'static str {
        match lang {
            Language::English => "Select a key below, then click 'Change Key Bind':",
            Language::Vietnamese => "Chọn một phím bên dưới, rồi bấm 'Đổi phím gán':",
        }
    }

    pub fn change_key_bind_btn(lang: Language, slot_num: u8) -> String {
        match lang {
            Language::English => format!("Change Key Bind (Button [{}])", slot_num),
            Language::Vietnamese => format!("Đổi phím gán (Nút [{}])", slot_num),
        }
    }

    pub fn rebind_title(lang: Language, btn_num: u8, mode_name: &str) -> String {
        match lang {
            Language::English => format!("Remap Button [{}] ({})", btn_num, mode_name),
            Language::Vietnamese => format!("Gán lại chức năng Nút [{}] ({})", btn_num, mode_name),
        }
    }

    pub fn step1_cat(lang: Language) -> &'static str {
        match lang {
            Language::English => "Step 1: Pick Category",
            Language::Vietnamese => "Bước 1: Chọn nhóm chức năng",
        }
    }

    pub fn step2_key(lang: Language) -> &'static str {
        match lang {
            Language::English => "Step 2: Pick Key or Action",
            Language::Vietnamese => "Bước 2: Chọn phím hoặc tác vụ",
        }
    }

    pub fn done_btn(lang: Language) -> &'static str {
        match lang {
            Language::English => "Done Keybinding",
            Language::Vietnamese => "Hoàn tất gán phím",
        }
    }

    pub fn cancel_btn(lang: Language) -> &'static str {
        match lang {
            Language::English => "Cancel",
            Language::Vietnamese => "Hủy bỏ",
        }
    }

    pub fn cat_mouse(lang: Language) -> &'static str {
        match lang {
            Language::English => "Standard Mouse Keys",
            Language::Vietnamese => "Phím chuột tiêu chuẩn",
        }
    }

    pub fn cat_keyboard(lang: Language) -> &'static str {
        match lang {
            Language::English => "Keyboard Keys",
            Language::Vietnamese => "Phím bàn phím",
        }
    }

    pub fn cat_dpi(lang: Language) -> &'static str {
        match lang {
            Language::English => "DPI & Special Keys",
            Language::Vietnamese => "DPI & Phím đặc biệt",
        }
    }

    pub fn cat_media(lang: Language) -> &'static str {
        match lang {
            Language::English => "Multimedia",
            Language::Vietnamese => "Đa phương tiện",
        }
    }

    pub fn cat_prod(lang: Language) -> &'static str {
        match lang {
            Language::English => "Productivity / Shortcuts",
            Language::Vietnamese => "Tiện ích & Lối tắt",
        }
    }

    pub fn settings_title(lang: Language) -> &'static str {
        match lang {
            Language::English => "Application Settings & Configuration",
            Language::Vietnamese => "Cài đặt Ứng dụng & Cấu hình",
        }
    }

    pub fn settings_desc(lang: Language) -> &'static str {
        match lang {
            Language::English => {
                "Configure display theme, interface scaling, storage paths, and hardware diagnostics."
            }
            Language::Vietnamese => {
                "Định cấu hình giao diện, độ phóng đại, đường dẫn lưu trữ và chẩn đoán phần cứng."
            }
        }
    }

    pub fn lang_section_title(lang: Language) -> &'static str {
        match lang {
            Language::English => "Language / Ngôn ngữ",
            Language::Vietnamese => "Ngôn ngữ / Language",
        }
    }

    pub fn lang_section_desc(lang: Language) -> &'static str {
        match lang {
            Language::English => "Select application display language:",
            Language::Vietnamese => "Chọn ngôn ngữ hiển thị giao diện:",
        }
    }

    pub fn theme_section_title(lang: Language) -> &'static str {
        match lang {
            Language::English => "Theme Palette",
            Language::Vietnamese => "Giao diện màu sắc",
        }
    }

    pub fn theme_section_desc(lang: Language) -> &'static str {
        match lang {
            Language::English => "Select an interface appearance theme:",
            Language::Vietnamese => "Chọn chủ đề hiển thị giao diện:",
        }
    }

    pub fn scale_section_title(lang: Language) -> &'static str {
        match lang {
            Language::English => "Display Scale Factor",
            Language::Vietnamese => "Độ phóng đại hiển thị",
        }
    }

    pub fn scale_section_desc(lang: Language) -> &'static str {
        match lang {
            Language::English => "Select a comfortable zoom level for your display resolution:",
            Language::Vietnamese => "Chọn mức thu phóng phù hợp với độ phân giải màn hình:",
        }
    }

    pub fn master_dir_title(lang: Language) -> &'static str {
        match lang {
            Language::English => "Master Data Directory (Multi-Install & Sync)",
            Language::Vietnamese => "Thư mục dữ liệu chính (Đồng bộ đa cài đặt)",
        }
    }

    pub fn master_dir_desc(lang: Language) -> &'static str {
        match lang {
            Language::English => {
                "Configure central storage path for profiles, cache, logs, and settings:"
            }
            Language::Vietnamese => {
                "Định cấu hình đường dẫn lưu trữ hồ sơ, bộ nhớ đệm, nhật ký và cài đặt:"
            }
        }
    }

    pub fn dev_debug_title(lang: Language) -> &'static str {
        match lang {
            Language::English => "Developer & Debug Mode",
            Language::Vietnamese => "Chế độ Gỡ lỗi & Lập trình viên",
        }
    }

    pub fn dev_debug_desc(lang: Language) -> &'static str {
        match lang {
            Language::English => "Launch dedicated native OS terminal to view live hardware logs and USB HID packets:",
            Language::Vietnamese => "Mở cửa sổ dòng lệnh hệ thống để xem nhật ký phần cứng và gói tin USB HID trực tiếp:",
        }
    }

    pub fn launch_terminal_btn(lang: Language) -> &'static str {
        match lang {
            Language::English => "Launch Live Debug Terminal",
            Language::Vietnamese => "Mở Cửa sổ Gỡ lỗi Trực tiếp",
        }
    }

    pub fn safety_title(lang: Language) -> &'static str {
        match lang {
            Language::English => "Hardware Safety & Experimental Features",
            Language::Vietnamese => "An toàn phần cứng & Tính năng thử nghiệm",
        }
    }

    pub fn safety_desc(lang: Language) -> &'static str {
        match lang {
            Language::English => {
                "Safety controls for SunplusIT SPCP6651B MCU and SPCP6662 optical sensor:"
            }
            Language::Vietnamese => {
                "Kiểm soát an toàn cho vi điều khiển SunplusIT SPCP6651B và cảm biến SPCP6662:"
            }
        }
    }

    pub fn allow_s6_label(lang: Language) -> &'static str {
        match lang {
            Language::English => "Allow Experimental Stage 6 DPI (Bypass Safety Clamp)",
            Language::Vietnamese => "Cho phép Stage 6 DPI thử nghiệm (Bỏ giới hạn an toàn)",
        }
    }

    pub fn about_title(lang: Language) -> &'static str {
        match lang {
            Language::English => "About E-DRA EM624 Configurator",
            Language::Vietnamese => "Giới thiệu E-DRA EM624 Configurator",
        }
    }

    pub fn about_desc(lang: Language) -> &'static str {
        match lang {
            Language::English => "E-DRA EM624 Mouse Configurator is an open-source, lightweight native driver and configuration utility built by reverse-engineering the stock SunplusIT firmware and USB HID protocol. It provides fast, offline profile management, calibrated 15-step DPI tuning, switch remapping, and EEPROM flashing without proprietary bloat or background services.",
            Language::Vietnamese => "E-DRA EM624 Mouse Configurator là trình điều khiển và cấu hình mã nguồn mở, siêu nhẹ được phát triển bằng kỹ thuật dịch ngược (reverse-engineering) firmware và giao thức USB HID của SunplusIT. Hỗ trợ quản lý hồ sơ ngoại tuyến, tinh chỉnh 15 mức DPI chuẩn xác, gán lại nút bấm phần cứng và nạp trực tiếp vào EEPROM mà không có phần mềm rác chạy ngầm.",
        }
    }

    pub fn author(lang: Language) -> &'static str {
        match lang {
            Language::English => "Author:",
            Language::Vietnamese => "Tác giả:",
        }
    }

    pub fn github_repo(lang: Language) -> &'static str {
        match lang {
            Language::English => "GitHub Repository:",
            Language::Vietnamese => "Kho lưu trữ GitHub:",
        }
    }
}
