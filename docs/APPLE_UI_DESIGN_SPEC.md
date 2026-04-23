# Apple HIG Style UI Redesign - Specification

## Overview
Redesign the RustTools NiceGUI application following Apple Human Interface Guidelines with a dark mode palette, glass effects, and refined typography.

---

## Design System

### Color Palette
| Token | Hex | Usage |
|-------|-----|-------|
| `bg` | `#1d1d1f` | Primary background |
| `card` | `#2d2d2d` | Card/surface background |
| `card-glass` | `rgba(45,45,45,0.7)` | Glass card overlay |
| `border` | `#48484a` | Subtle borders |
| `border-light` | `#636366` | Hover/active borders |
| `text-primary` | `#f5f5f7` | Primary text |
| `text-secondary` | `#a1a1a6` | Secondary text |
| `text-tertiary` | `#6e6e73` | Tertiary/placeholder |
| `accent-blue` | `#0a84ff` | Primary accent |
| `accent-green` | `#30d158` | Success/complete |
| `accent-orange` | `#ff9f0a` | Warning/pending |
| `accent-red` | `#ff453a` | Error/destructive |

### Typography
- **Font Stack**: `-apple-system, BlinkMacSystemFont, "SF Pro Display", "SF Pro Text", system-ui`
- **Title**: 28px, 600 weight
- **Heading**: 20px, 600 weight
- **Body**: 14px, 400 weight
- **Caption**: 12px, 400 weight

### Spacing System
- Base unit: 4px
- Component gap: 16px (gap-4)
- Section gap: 24px (gap-6)
- Page padding: 24px (p-6)

### Border Radius
| Token | Value | Usage |
|-------|-------|-------|
| `sm` | 8px | Buttons, inputs |
| `md` | 12px | Cards |
| `lg` | 16px | Large cards |
| `xl` | 20px | Hero cards |

### Shadows
- **Card**: `0 4px 16px rgba(0,0,0,0.4)`
- **Glass**: `0 4px 24px rgba(0,0,0,0.4), inset 0 1px 0 rgba(255,255,255,0.05)`

### Effects
- **Glass blur**: `backdrop-blur: 20px`
- **Hover transitions**: 200ms ease

---

## File Structure

```
RustTools/
├── core/
│   └── ui/
│       ├── __init__.py           # Package init
│       ├── apple_theme.py        # Design tokens + CSS injection
│       └── components.py        # Reusable components
├── modules/
│   └── yolo/
│       └── pages/
│           ├── inference.py      # [REDESIGN]
│           ├── training.py        # [REDESIGN]
│           ├── annotation.py      # [REDESIGN]
│           ├── video.py           # [REDESIGN]
│           └── results.py         # [REDESIGN]
├── main.py                       # [MODIFY] Add theme init
└── docs/
    └── APPLE_UI_DESIGN_SPEC.md   # This file
```

---

## Page Redesigns

### 1. Hub Page (`/`)

**Layout**: Full-screen centered layout with hero section

**Structure**:
```
┌────────────────────────────────────────────────────┐
│                  [Background: #1d1d1f]            │
│                                                    │
│              ┌──────────────────────┐              │
│              │    🔧 RustTools      │              │
│              │   YOLO 目标检测工具   │              │
│              │                      │              │
│              │  [Device Badge]      │              │
│              │                      │              │
│              │  ┌────┐ ┌────┐       │              │
│              │  │YOLO│ │ ?? │ ...   │              │
│              │  └────┘ └────┘       │              │
│              └──────────────────────┘              │
└────────────────────────────────────────────────────┘
```

**Components**:
- Large centered glass card (max-width: 800px)
- App title with emoji icon
- Subtitle in secondary text color
- Device info badge (GPU/CPU indicator)
- Module cards in responsive row

**Module Cards**:
- Border-radius: 20px
- Hover: scale(1.02) + lighter border
- Icon (emoji), name, description, version

---

### 2. YOLO Index Page (`/yolo`)

**Layout**: Tabbed interface with sidebar navigation

**Structure**:
```
┌────────────────────────────────────────────────────┐
│ YOLO          目标检测 · 训练 · 标注    [Device]   │
├────────────────────────────────────────────────────┤
│ ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐     │
│ │ 推理 │ │ 训练 │ │ 标注 │ │ 视频 │ │ 结果 │     │
│ └──────┘ └──────┘ └──────┘ └──────┘ └──────┘     │
├────────────────────────────────────────────────────┤
│                                                    │
│              [Active Tab Content]                 │
│                                                    │
└────────────────────────────────────────────────────┘
```

**Components**:
- Top bar: Title + subtitle + device badge
- Tab bar: Horizontal pill-style tabs with icons
- Active tab indicator with blue underline
- Content area with consistent padding

---

### 3. Inference Page (`/yolo/inference`)

**Layout**: Two-column split (controls | preview)

**Structure**:
```
┌────────────────────────────────────────────────────┐
│ ┌──────────────┐  ┌─────────────────────────────┐ │
│ │  推理控制     │  │  检测结果                   │ │
│ │              │  │                             │ │
│ │  [模型选择]   │  │  ┌─────────────────────┐   │ │
│ │              │  │  │                     │   │ │
│ │  置信度       │  │  │   [Image Preview]   │   │ │
│ │  [━━━━━━●──] │  │  │                     │   │ │
│ │              │  │  └─────────────────────┘   │ │
│ │  [上传图片]   │  │                             │ │
│ │              │  │  检测到 X 个目标             │ │
│ │  [开始检测]   │  │  class1, class2, ...        │ │
│ └──────────────┘  └─────────────────────────────┘ │
└────────────────────────────────────────────────────┘
```

**Left Panel** (1/3 width):
- Section header: "推理控制"
- Model select dropdown (Apple-styled)
- Confidence slider with live value display
- File upload with drag indicator
- Action button (primary blue)

**Right Panel** (2/3 width):
- Section header: "检测结果"
- Image preview area with rounded corners
- Result statistics below image

**Colors**:
- Card backgrounds: #2d2d2d
- Accent buttons: #0a84ff
- Status text: #30d158 (success)

---

### 4. Training Page (`/yolo/training`)

**Layout**: Two-column (config | status)

**Structure**:
```
┌────────────────────────────────────────────────────┐
│ ┌──────────────┐  ┌─────────────────────────────┐ │
│ │  训练配置     │  │  训练状态                    │ │
│ │              │  │                             │ │
│ │  [模型    ▼] │  │  ┌────┐ ┌────┐ ┌────┐     │ │
│ │              │  │  │metric│ │metric│ │metric│ │ │
│ │  数据集路径   │  │  └────┘ └────┘ └────┘     │ │
│ │  [________]  │  │                             │ │
│ │              │  │  训练历史                    │ │
│ │  轮次 [100]  │  │  ┌─────────────────────┐   │ │
│ │  [━━━━━━●──] │  │  │ Run 001  ● 运行中   │   │ │
│ │              │  │  │ Run 000  ● 完成     │   │ │
│ │  批量  [16]  │  │  └─────────────────────┘   │ │
│ │  [━━━━●────] │  │                             │ │
│ │              │  │                             │ │
│ │  尺寸  [640] │  │                             │ │
│ │              │  │                             │ │
│ │  [开始训练]   │  │                             │ │
│ └──────────────┘  └─────────────────────────────┘ │
└────────────────────────────────────────────────────┘
```

**Left Panel**:
- Section header: "训练配置"
- Model select
- Dataset YAML input
- Sliders: Epochs (1-300), Batch (1-64), Image Size (320-1280)
- Start training button (green: #30d158)

**Right Panel**:
- Section header: "训练状态"
- 3 metric cards: Epochs, mAP, Loss
- Training runs table with status badges

**Metric Cards**:
- Border-radius: 12px
- Value: 3xl bold white
- Label: sm gray-400
- Optional delta indicator

---

### 5. Annotation Page (`/yolo/annotation`)

**Layout**: Full-width with centered content

**Structure**:
```
┌────────────────────────────────────────────────────┐
│              ┌─────────────────────────┐          │
│              │     数据标注              │          │
│              │   绘制 bounding box      │          │
│              │                          │          │
│              │     🖼️                   │          │
│              │                          │          │
│              │   [功能开发中...]         │          │
│              │                          │          │
│              │   ┌────────────────┐    │          │
│              │   │ 开始标注       │    │          │
│              │   └────────────────┘    │          │
│              └─────────────────────────┘          │
└────────────────────────────────────────────────────┘
```

**Components**:
- Empty state with icon
- "功能开发中" message
- Disabled button (for now)

---

### 6. Video Page (`/yolo/video`)

**Layout**: Similar to inference (controls | preview)

**Structure**:
```
┌────────────────────────────────────────────────────┐
│ ┌──────────────┐  ┌─────────────────────────────┐ │
│ │  视频推理     │  │  视频预览                    │ │
│ │              │  │                             │ │
│ │  [模型选择]   │  │                             │ │
│ │              │  │                             │ │
│ │  [上传视频]   │  │                             │ │
│ │              │  │                             │ │
│ │  [开始处理]   │  │                             │ │
│ └──────────────┘  └─────────────────────────────┘ │
└────────────────────────────────────────────────────┘
```

---

### 7. Results Page (`/yolo/results`)

**Layout**: Two-column (training | models)

**Structure**:
```
┌────────────────────────────────────────────────────┐
│ ┌──────────────────────┐  ┌─────────────────────┐  │
│ │  训练历史            │  │  缓存模型            │  │
│ │                      │  │                     │  │
│ │  ┌────────────────┐  │  │  ┌───────────────┐ │  │
│ │  │ Run 001 ●完成 │  │  │  │  yolo11n.pt   │ │  │
│ │  │ 100 epochs    │  │  │  │  6.2 MB       │ │  │
│ │  └────────────────┘  │  │  └───────────────┘ │  │
│ │                      │  │                     │  │
│ │  ┌────────────────┐  │  │  ┌───────────────┐ │  │
│ │  │ Run 000 ●失败 │  │  │  │  yolo8n.pt   │ │  │
│ │  └────────────────┘  │  │  └───────────────┘ │  │
│ └──────────────────────┘  └─────────────────────┘  │
└────────────────────────────────────────────────────┘
```

**Training History Cards**:
- Status indicator (green/red dot)
- Run ID, epochs, date
- Action buttons (view, delete)

**Model Cards**:
- Model name, size
- Download/delete actions

---

## Component Usage

### Initialization
```python
from core.ui import init_apple_ui

# In main.py before any UI code
init_apple_ui()
```

### Creating Pages
```python
from core.ui.components import (
    apple_button,
    apple_select,
    apple_slider,
    apple_input,
    glass_card,
    metric_card,
    two_column_layout,
)

def render() -> None:
    with ui.column().classes("w-full p-6 gap-6"):
        page_header("Title", "Subtitle")
        
        with two_column_layout(left_panel, right_panel):
            pass
```

---

## Risk Assessment

| Risk | Magnitude | Mitigation |
|------|-----------|------------|
| NiceGUI Tailwind conflicts | Medium | Use explicit classes, avoid !important |
| Component reusability | Low | Components are stateless callbacks |
| Backdrop blur performance | Low | Only on glass cards, not everywhere |
| Color contrast accessibility | Low | #f5f5f7 on #1d1d1f meets WCAG AA |

---

## Convergence Criteria

1. ✅ All pages use consistent color tokens
2. ✅ Cards have uniform border-radius (12px)
3. ✅ Buttons use blue/green/orange variants
4. ✅ Forms use consistent input styling
5. ✅ Glass effects on hero/special sections
6. ✅ No raw color values in page code
