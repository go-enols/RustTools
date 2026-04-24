/// 应用路由状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    Welcome,
    Hub,
    Project,
    Annotation,
    Training,
    Video,
    Desktop,
    Device,
    Settings,
}

impl std::fmt::Display for Route {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Route::Welcome => write!(f, "欢迎"),
            Route::Hub => write!(f, "总览"),
            Route::Project => write!(f, "项目管理"),
            Route::Annotation => write!(f, "图像标注"),
            Route::Training => write!(f, "模型训练"),
            Route::Video => write!(f, "视频推理"),
            Route::Desktop => write!(f, "桌面捕获"),
            Route::Device => write!(f, "设备信息"),
            Route::Settings => write!(f, "环境设置"),
        }
    }
}

impl Route {
    #[allow(dead_code)]
    pub fn icon(&self) -> &'static str {
        ""
    }

    #[allow(dead_code)]
    pub fn description(&self) -> &'static str {
        match self {
            Route::Welcome => "RustTools 欢迎页",
            Route::Hub => "功能总览与快捷入口",
            Route::Project => "创建与管理YOLO项目",
            Route::Annotation => "图像标注与数据集制作",
            Route::Training => "模型训练与超参调优",
            Route::Video => "视频文件推理分析",
            Route::Desktop => "实时屏幕捕获检测",
            Route::Device => "GPU/CPU设备信息",
            Route::Settings => "Python环境与依赖管理",
        }
    }
}
