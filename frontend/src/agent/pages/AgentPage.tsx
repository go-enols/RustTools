/**
 * Agent 模块顶层路由页面
 *
 * 作为 AI Agent 独立模块的入口组件，与 YOLO 工具页面完全解耦。
 * 内部渲染 AgentIDE 工作区。
 */
import AgentIDE from "../components/AgentIDE";

export default function AgentPage() {
  return <AgentIDE />;
}
