import { useNavigate } from "react-router-dom";
import { Camera, Sparkles, Brain } from "lucide-react";

export default function Welcome() {
  const navigate = useNavigate();

  return (
    <div className="min-h-full flex flex-col items-center justify-center p-8 relative overflow-hidden">
      {/* 背景装饰 */}
      <div className="absolute top-20 left-20 w-72 h-72 bg-blue-500/5 rounded-full blur-3xl" />
      <div className="absolute bottom-20 right-20 w-96 h-96 bg-purple-500/5 rounded-full blur-3xl" />

      <div className="relative z-10 text-center">
        {/* Logo */}
        <div className="w-20 h-20 rounded-2xl bg-gradient-to-br from-blue-500 via-purple-500 to-pink-500 flex items-center justify-center mx-auto mb-6 shadow-lg shadow-purple-500/20">
          <Sparkles className="w-10 h-10 text-white" />
        </div>

        <h1 className="text-4xl font-bold text-gray-900 dark:text-white mb-3 tracking-tight">
          RustTools
        </h1>
        <p className="text-base text-gray-500 dark:text-gray-400 mb-10 max-w-md mx-auto leading-relaxed">
          一站式高性能 Rust 工具箱
        </p>

        <div className="flex gap-4 justify-center">
          <ModuleCard
            icon={<Camera className="w-5 h-5" />}
            title="YOLO 视觉"
            desc="目标检测 / 标注 / 训练 / 推理"
            gradient="from-pink-500 to-rose-500"
            onClick={() => navigate("/hub")}
          />
          <ModuleCard
            icon={<Brain className="w-5 h-5" />}
            title="AI 助手"
            desc="智能对话 / 代码生成 / 任务编排"
            gradient="from-blue-500 to-cyan-500"
            onClick={() => navigate("/agent")}
          />
        </div>
      </div>

      <p className="absolute bottom-6 text-xs text-gray-300 dark:text-gray-700">
        RustTools v1.0.0
      </p>
    </div>
  );
}

function ModuleCard({
  icon,
  title,
  desc,
  gradient,
  onClick,
}: {
  icon: React.ReactNode;
  title: string;
  desc: string;
  gradient: string;
  onClick: () => void;
}) {
  return (
    <button
      onClick={onClick}
      className="group w-56 h-40 rounded-2xl bg-white dark:bg-surface-dark border border-gray-100 dark:border-gray-800 shadow-sm hover:shadow-xl hover:-translate-y-1 transition-all duration-300 p-5 text-left flex flex-col relative overflow-hidden"
    >
      <div className={`w-10 h-10 rounded-xl bg-gradient-to-br ${gradient} flex items-center justify-center text-white mb-3 shadow-md group-hover:scale-110 transition-transform duration-300`}>
        {icon}
      </div>
      <h3 className="text-sm font-semibold text-gray-900 dark:text-white mb-1">{title}</h3>
      <p className="text-xs text-gray-500 dark:text-gray-400 leading-relaxed">{desc}</p>
      <div className={`absolute -bottom-6 -right-6 w-24 h-24 rounded-full bg-gradient-to-br ${gradient} opacity-0 group-hover:opacity-10 transition-opacity duration-500 blur-xl`} />
    </button>
  );
}
