#!/usr/bin/env python3
"""
诊断YOLO模型输出格式
用于检测推理帧率为0的问题
"""

import onnx
import numpy as np

def analyze_yolo_model(model_path):
    """分析YOLO模型的输出格式"""
    print("=" * 60)
    print("YOLO模型输出格式诊断")
    print("=" * 60)
    print()
    
    # 加载模型
    model = onnx.load(model_path)
    
    # 获取模型信息
    print(f"模型文件: {model_path}")
    print(f"IR版本: {model.ir_version}")
    print(f"Producer: {model.producer_name} {model.producer_version}")
    print(f"Opset版本: {model.opset_import[0].version}")
    print()
    
    # 分析输入
    print("模型输入:")
    for input in model.graph.input:
        shape = [dim.dim_value if dim.dim_value > 0 else '?' for dim in input.type.tensor_type.shape.dim]
        print(f"  {input.name}: {shape}, dtype: {input.type.tensor_type.elem_type}")
    print()
    
    # 分析输出
    print("模型输出:")
    for output in model.graph.output:
        shape = [dim.dim_value if dim.dim_value > 0 else '?' for dim in output.type.tensor_type.shape.dim]
        print(f"  {output.name}: {shape}, dtype: {output.type.tensor_type.elem_type}")
    print()
    
    # 尝试确定YOLO版本和格式
    if len(model.graph.output) > 0:
        output_shape = [dim.dim_value if dim.dim_value > 0 else '?' for dim in model.graph.output[0].type.tensor_type.shape.dim]
        print("=" * 60)
        print("输出格式分析")
        print("=" * 60)
        
        if len(output_shape) == 3:
            batch, features, boxes = output_shape
            print(f"检测到格式: [batch={batch}, features={features}, boxes={boxes}]")
            
            if features == 84:
                print("  → 这是YOLOv8格式")
                print("  → 84 = 4 (bbox: cx, cy, w, h) + 80 (COCO classes)")
                print("  → 如果你的模型只有4个类别,需要确认是否重新导出")
            elif features == 85:
                print("  → 这是YOLOv5格式")
                print("  → 85 = 4 (bbox: x, y, w, h) + 1 (obj conf) + 80 (COCO classes)")
            elif features == 5:
                print("  → 这可能是自定义的4类模型")
                print("  → 5 = 4 (bbox) + 1 (class) 或 4 (bbox) + 1 (confidence)")
            else:
                print(f"  → 未知格式, features={features}")
                print(f"  → 可能的解释:")
                print(f"    - 如果只有4个类别,features应该是 4+4=8 或 4+1=5")
                print(f"    - 如果是80类模型,features应该是 4+80=84 或 4+1+80=85")
        else:
            print(f"  → 输出维度数量: {len(output_shape)}")
            print(f"  → 这不是标准的YOLO格式")
    
    print()
    print("=" * 60)
    print("诊断结论")
    print("=" * 60)
    print()
    
    # 给出建议
    if len(output_shape) == 3 and features == 84:
        print("✓ 模型看起来是YOLOv8格式")
        print()
        print("建议:")
        print("1. 如果你的野生动物模型只有4个类别:")
        print("   - 使用ultralytics重新导出模型:")
        print("     from ultralytics import YOLO")
        print("     model = YOLO('yolo11n.pt')")
        print("     # 训练你的4类模型")
        print("     model.export(format='onnx', imgsz=640)")
        print()
        print("2. 或者修改代码以支持YOLOv8格式:")
        print("   - 类别分数可能是原始logits,需要应用sigmoid")
        print("   - 确保使用正确的坐标转换公式")
    elif len(output_shape) == 3 and features == 5:
        print("✓ 模型看起来是4类格式")
        print()
        print("代码需要调整:")
        print("- 如果features=5,可能格式是 [batch, 5, boxes]")
        print("- 5可能表示 4(bbox) + 1(class) 或 4(bbox) + 1(conf)")
        print("- 需要根据实际情况调整解析逻辑")
    else:
        print("⚠ 无法确定模型格式")
        print()
        print("请提供更多信息:")
        print("- 模型来源 (YOLOv8? YOLOv5? 自定义?)")
        print("- 训练的类别数量")
        print("- 模型导出时的配置")
    
    print()
    print("=" * 60)

if __name__ == "__main__":
    model_path = r"C:\Users\25751\Desktop\african-wildlife\yolo11n.onnx"
    analyze_yolo_model(model_path)
