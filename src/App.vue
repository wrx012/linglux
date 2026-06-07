<script setup lang="ts">
import type { Component } from "vue";
import { computed, onMounted, onUnmounted, reactive, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import {
  Aperture,
  AudioLines,
  Box,
  Camera,
  Clapperboard,
  Cpu,
  Download,
  Eye,
  EyeOff,
  Film,
  Folder,
  Globe2,
  Grid2x2,
  Image as ImageIcon,
  KeyRound,
  Layers,
  MousePointer2,
  Plus,
  Save,
  Server,
  Settings,
  SlidersHorizontal,
  Sparkles,
  Smartphone,
  Trash2,
  Type as TypeIcon,
  Upload,
  Video,
  WandSparkles,
  X,
  Clock,
} from "@lucide/vue";
import lingluxLogo from "./assets/linglux-logo-no-text.png";

type ApiProviderValue = "openai" | "openrouter" | "custom";

interface ApiKeySettings {
  provider: string;
  baseUrl: string;
  apiKey: string;
}

interface SupportNavItem {
  label: string;
  meta?: "saved" | "empty";
  icon: Component;
  action?: "api-key";
}

type CameraCategory = "film" | "digital" | "photo" | "phone";

interface CameraTypeTab {
  label: string;
  value: CameraCategory;
  icon: Component;
}

interface CameraBodyProfile {
  id: string;
  category: CameraCategory;
  label: string;
  year: string;
  format: string;
  sensor: string;
  description: string;
  works: string[];
  promptCue: string;
}

interface CameraPreset {
  label: string;
  value: string;
  lens: string;
  focalLength: string;
  aperture: string;
  effect: string;
}

interface LensProfile {
  label: string;
  value: string;
  description: string;
  promptCue: string;
}

interface LensEffect {
  label: string;
  value: string;
  description: string;
  promptCue: string;
}

const API_KEY_STORAGE_KEY = "linglux-api-key-settings";
const CAMERA_PROMPT_PREFIX = "Camera:";

const apiProviderOptions: Array<{ label: string; value: ApiProviderValue; baseUrl: string }> = [
  { label: "OpenAI", value: "openai", baseUrl: "https://api.openai.com/v1" },
  { label: "OpenRouter", value: "openrouter", baseUrl: "https://openrouter.ai/api/v1" },
  { label: "自定义", value: "custom", baseUrl: "https://api.openai.com/v1" },
];

const defaultApiKeySettings: ApiKeySettings = {
  provider: "openai",
  baseUrl: "https://api.openai.com/v1",
  apiKey: "",
};

const cameraTypeTabs: CameraTypeTab[] = [
  { label: "胶片机", value: "film", icon: Film },
  { label: "数字机", value: "digital", icon: Clapperboard },
  { label: "照相机", value: "photo", icon: Camera },
  { label: "手机", value: "phone", icon: Smartphone },
];

const cameraBodies: CameraBodyProfile[] = [
  {
    id: "arri-35-iii",
    category: "film",
    label: "阿莱35-III型电影摄影机",
    year: "1979",
    format: "Super 35mm Film",
    sensor: "35mm胶片 (24.89x18.66mm感光面积)",
    description:
      "阿莱35-III型电影摄影机，好莱坞黄金年代主力机型，采用PL卡口，支持4齿孔/3齿孔走片，最高速度40fps。代表作：《星球大战》三部曲、《夺宝奇兵》系列、《E.T.外星人》。",
    works: ["星球大战三部曲", "夺宝奇兵", "E.T.外星人", "第三类接触"],
    promptCue: "shot on ARRI 35-III Super 35mm film, classic late-70s cinema texture",
  },
  {
    id: "arriflex-435",
    category: "film",
    label: "ARRIFLEX 435",
    year: "1995",
    format: "Super 35mm Film",
    sensor: "35mm胶片 (高速走片)",
    description: "ARRIFLEX 435 是高速运动、广告和特效摄影常用胶片机，机身稳定，最高可达150fps，适合清晰的动作分解和慢动作质感。",
    works: ["黑客帝国", "角斗士", "指环王"],
    promptCue: "shot on ARRIFLEX 435 Super 35mm film, clean high-speed action cinema look",
  },
  {
    id: "bolex-h16",
    category: "film",
    label: "Bolex H16",
    year: "1935",
    format: "16mm Film",
    sensor: "16mm胶片 (约10.26x7.49mm)",
    description: "Bolex H16 是经典16mm摄影机，机动轻巧、颗粒明显，适合实验电影、纪录影像和复古家庭电影质感。",
    works: ["实验短片", "独立纪录片", "影像日记"],
    promptCue: "shot on Bolex H16 16mm film, tactile grain and intimate handheld texture",
  },
  {
    id: "arri-alexa-classic",
    category: "digital",
    label: "阿莱艾丽莎经典型数字电影机",
    year: "2010",
    format: "Super 35 (2.8K)",
    sensor: "ARRI ALEV III CMOS传感器 (23.76x13.37mm), 2880x1620有效像素, 14+档动态范围",
    description:
      "阿莱艾丽莎经典型数字电影机，数字电影革命开创者，奠定ARRI色彩科学标准。支持ProRes和ARRIRAW。代表作：《雨果》、《生命之树》、《艺术家》、《乌云背后的幸福线》。",
    works: ["雨果", "生命之树", "艺术家", "乌云背后的幸福线", "林肯"],
    promptCue: "shot on ARRI ALEXA Classic, natural highlight rolloff and filmic digital color",
  },
  {
    id: "sony-venice-2",
    category: "digital",
    label: "Sony VENICE 2",
    year: "2021",
    format: "Full Frame 8.6K",
    sensor: "36x24mm全画幅CMOS, 16档动态范围, 双原生ISO",
    description: "Sony VENICE 2 面向高端剧集和电影摄影，提供全画幅浅景深、宽动态范围和灵活的机内录制格式。",
    works: ["壮志凌云：独行侠", "阿凡达：水之道", "高端剧集制作"],
    promptCue: "shot on Sony VENICE 2 full-frame, polished modern cinema depth and color",
  },
  {
    id: "red-v-raptor",
    category: "digital",
    label: "RED V-RAPTOR 8K VV",
    year: "2021",
    format: "Vista Vision 8K",
    sensor: "40.96x21.60mm 8K VV CMOS, 17+档动态范围",
    description: "RED V-RAPTOR 8K VV 以高分辨率、高帧率和RAW工作流见长，适合广告、MV、动作摄影和需要后期重构画面的制作。",
    works: ["广告片", "音乐录影带", "动作短片"],
    promptCue: "shot on RED V-RAPTOR 8K VV, crisp high-resolution commercial cinema look",
  },
  {
    id: "nikon-f3",
    category: "photo",
    label: "Nikon F3",
    year: "1980",
    format: "35mm Film",
    sensor: "-",
    description: "传奇专业胶片单反，NASA太空任务用机。",
    works: [],
    promptCue: "shot on Nikon F3 35mm film, tactile still-photo realism and subtle grain",
  },
  {
    id: "leica-m6",
    category: "photo",
    label: "Leica M6",
    year: "1984",
    format: "35mm Film",
    sensor: "-",
    description: "经典旁轴胶片相机，机身小巧、取景直接，适合纪实、人文和街头摄影气质。",
    works: ["街头摄影", "纪实摄影", "旅行影像"],
    promptCue: "shot on Leica M6 35mm film, candid documentary mood and compact rangefinder framing",
  },
  {
    id: "hasselblad-500cm",
    category: "photo",
    label: "Hasselblad 500C/M",
    year: "1970",
    format: "6x6 Medium Format Film",
    sensor: "120中画幅胶片 (56x56mm)",
    description: "模块化中画幅胶片相机，方画幅、细腻层次和高解析力是其标志，适合静物、人像和时装摄影。",
    works: ["阿波罗计划影像", "时装摄影", "棚拍人像"],
    promptCue: "shot on Hasselblad 500C/M medium format film, square-frame clarity and smooth tonal depth",
  },
  {
    id: "iphone-15-pro-max",
    category: "phone",
    label: "iPhone 15 Pro Max",
    year: "2023",
    format: "-",
    sensor: "48MP Main",
    description: "5倍四棱镜长焦、ProRAW、Log视频。",
    works: [],
    promptCue: "shot on iPhone 15 Pro Max, mobile ProRAW clarity and Apple Log video response",
  },
  {
    id: "xiaomi-14-ultra",
    category: "phone",
    label: "Xiaomi 14 Ultra",
    year: "2024",
    format: "1-inch type mobile sensor",
    sensor: "50MP LYT-900主摄, 可变光圈",
    description: "徕卡影像调校、多焦段移动摄影系统，适合高对比街景、夜景和移动纪实感。",
    works: ["移动街拍", "夜景视频", "旅行影像"],
    promptCue: "shot on Xiaomi 14 Ultra, Leica-tuned mobile contrast and high-detail night capture",
  },
  {
    id: "galaxy-s24-ultra",
    category: "phone",
    label: "Samsung Galaxy S24 Ultra",
    year: "2024",
    format: "-",
    sensor: "200MP Wide",
    description: "高像素主摄与多焦段长焦组合，画面锐利，适合清晰、明亮、偏现代的移动视频风格。",
    works: ["移动长焦", "城市夜景", "旅行视频"],
    promptCue: "shot on Samsung Galaxy S24 Ultra, sharp mobile telephoto detail and vivid modern color",
  },
];

const lensOptions: LensProfile[] = [
  {
    label: "Zeiss Master Prime",
    value: "Zeiss Master Prime",
    description: "极致锐利，几乎无炫光，专业电影制作首选",
    promptCue: "ultra-sharp Zeiss Master Prime optics with minimal flare",
  },
  {
    label: "Cooke S4/i",
    value: "Cooke S4/i",
    description: "暖调肤色、柔和反差，带有经典 Cooke Look",
    promptCue: "warm Cooke S4/i rendering with gentle contrast",
  },
  {
    label: "ARRI Signature Prime",
    value: "ARRI Signature Prime",
    description: "现代大画幅镜头，肤色自然，焦外柔顺",
    promptCue: "natural ARRI Signature Prime color with smooth falloff",
  },
  {
    label: "Canon K35",
    value: "Canon K35",
    description: "复古低反差、暖色高光和柔和边缘",
    promptCue: "vintage Canon K35 warmth, lower contrast and soft highlight bloom",
  },
  {
    label: "Leica Summilux-C",
    value: "Leica Summilux-C",
    description: "高微反差、色彩克制，适合精致商业片",
    promptCue: "refined Leica Summilux-C micro-contrast and controlled color",
  },
];

const focalLengthOptions = ["12mm", "18mm", "24mm", "35mm", "50mm", "75mm", "100mm"];
const apertureOptions = ["f/1.3", "f/1.4", "f/2", "f/2.8", "f/4", "f/5.6"];

const lensEffects: LensEffect[] = [
  {
    label: "散景",
    value: "bokeh",
    description: "背景柔化、主体分离，强调电影感景深",
    promptCue: "cinematic bokeh with strong subject separation",
  },
  {
    label: "眩光",
    value: "flare",
    description: "轻微镜头眩光，增强高光方向感",
    promptCue: "subtle controlled lens flare in highlights",
  },
  {
    label: "手持",
    value: "handheld",
    description: "轻微手持运动，带有现场纪录气息",
    promptCue: "subtle handheld camera energy",
  },
];

const cameraPresets: CameraPreset[] = [
  { label: "默认", value: "default", lens: "Zeiss Master Prime", focalLength: "12mm", aperture: "f/1.3", effect: "bokeh" },
  { label: "电影宽景", value: "wide-cinema", lens: "ARRI Signature Prime", focalLength: "18mm", aperture: "f/2.8", effect: "flare" },
  { label: "人像浅景深", value: "portrait-depth", lens: "Cooke S4/i", focalLength: "75mm", aperture: "f/1.4", effect: "bokeh" },
  { label: "手持纪录", value: "documentary", lens: "Canon K35", focalLength: "35mm", aperture: "f/4", effect: "handheld" },
];

const prompt = ref("Cyberpunk commander, neon light, hyperreal skin, cinematic rim light...");
const negativePrompt = ref("bad proportions, extra limbs, low quality render...");
const renderSteps = ref(25);
const cfgScale = ref(6.5);
const selectedMotion = ref("medium");
const isCameraPanelOpen = ref(false);
const selectedCameraCategory = ref<CameraCategory>("film");
const selectedCameraBodyId = ref("arri-35-iii");
const selectedCameraPreset = ref("default");
const selectedLens = ref("Zeiss Master Prime");
const selectedFocalLength = ref("12mm");
const selectedAperture = ref("f/1.3");
const selectedLensEffect = ref("bokeh");
const hostStatus = ref("Tauri host ready");
const isApiKeyPanelOpen = ref(false);
const isSavingApiKey = ref(false);
const isClearingApiKey = ref(false);
const showApiKey = ref(false);
const apiKeyMessage = ref("正在读取 API Key 设置...");
const apiKeySettings = ref<ApiKeySettings>({ ...defaultApiKeySettings });
const apiKeyDraft = ref<ApiKeySettings>({ ...defaultApiKeySettings });

type ModelGroup = "image" | "video";
type NodeType = "source" | "text" | "image" | "video" | "world3d" | "audio" | "storyboard" | "aiApp" | "import";
type NodeSide = "left" | "right";

interface CanvasNode {
  id: string;
  type: NodeType;
  x: number;
  y: number;
  width: number;
  height: number;
  status: string;
  assetName?: string;
  modelId?: string;
}

interface DragState {
  id: string;
  pointerId: number;
  startX: number;
  startY: number;
  originX: number;
  originY: number;
}

interface CanvasPanState {
  pointerId: number;
  startX: number;
  startY: number;
  originX: number;
  originY: number;
}

interface NodeContextMenuState {
  nodeId: string;
  x: number;
  y: number;
}

interface Point {
  x: number;
  y: number;
}

interface ModelOption {
  id: string;
  label: string;
  subtitle: string;
  baseOutputTokens: number;
}

interface NodeDefinition {
  label: string;
  title: string;
  role: string;
  description: string;
  icon: Component;
  width: number;
  height: number;
  inputOffset?: number;
  outputOffset?: number;
  modelGroup?: ModelGroup;
}

interface NodePaletteItem {
  label: string;
  type: NodeType;
  icon: Component;
}

interface NodePaletteSection {
  label: string;
  items: NodePaletteItem[];
}

interface WorkflowEdge {
  id: string;
  from: string;
  to: string;
}

interface WireDisplay {
  id: string;
  path: string;
  signal: Point;
  signalClass: string;
}

const canvasBounds = {
  width: 1160,
  height: 760,
  padding: 16,
  topPadding: 64,
};

const CANVAS_VIEWPORT_GUTTER = 40;
const CANVAS_TOOLBAR_SPACE = 86;
const MIN_CANVAS_SCALE = 0.58;
const NODE_CONTEXT_MENU_WIDTH = 178;
const NODE_CONTEXT_MENU_HEIGHT = 44;
const NODE_CONTEXT_MENU_MARGIN = 8;
const MIN_UI_SCALE_WIDTH = 960;
const MIN_UI_SCALE_HEIGHT = 640;
const TARGET_UI_SCALE_WIDTH = 1200;
const TARGET_UI_SCALE_HEIGHT = 800;
const MAX_UI_SCALE = 1.25;

const nodeDefinitions: Record<NodeType, NodeDefinition> = {
  source: {
    label: "上传资源",
    title: "IMAGE / 图像输入",
    role: "Source",
    description: "从本地导入参考图、角色图或关键帧资源。",
    icon: Upload,
    width: 280,
    height: 248,
    outputOffset: 124,
  },
  text: {
    label: "文本",
    title: "TEXT / 文本提示",
    role: "Input",
    description: "编写镜头提示词、对白、旁白或分镜描述。",
    icon: TypeIcon,
    width: 280,
    height: 218,
    outputOffset: 109,
  },
  image: {
    label: "图片",
    title: "AI IMAGE / 智能生成",
    role: "Process",
    description: "生成、增强或重绘单帧画面与视觉资产。",
    icon: ImageIcon,
    width: 320,
    height: 620,
    inputOffset: 120,
    outputOffset: 120,
    modelGroup: "image",
  },
  video: {
    label: "视频",
    title: "VIDEO / 视频扩散",
    role: "Target",
    description: "把关键帧扩展为动态镜头，控制时长与运动幅度。",
    icon: Video,
    width: 320,
    height: 620,
    inputOffset: 120,
    modelGroup: "video",
  },
  world3d: {
    label: "3D 世界",
    title: "3D WORLD / 场景空间",
    role: "Scene",
    description: "搭建空间、相机路径和场景约束，用于镜头生成。",
    icon: Globe2,
    width: 280,
    height: 238,
    outputOffset: 119,
  },
  audio: {
    label: "音频",
    title: "AUDIO / 声音轨道",
    role: "Sound",
    description: "添加音乐、音效、旁白或口型同步参考。",
    icon: AudioLines,
    width: 280,
    height: 218,
    outputOffset: 109,
  },
  storyboard: {
    label: "分镜格子",
    title: "STORYBOARD / 分镜格子",
    role: "Layout",
    description: "组织镜头节奏、关键帧顺序和转场关系。",
    icon: Grid2x2,
    width: 300,
    height: 256,
    inputOffset: 128,
    outputOffset: 128,
  },
  aiApp: {
    label: "AI 应用",
    title: "AI APP / 自动化应用",
    role: "Agent",
    description: "接入批处理、风格迁移、字幕或增强类 AI 应用。",
    icon: Layers,
    width: 300,
    height: 256,
    inputOffset: 128,
    outputOffset: 128,
  },
  import: {
    label: "从作品导入",
    title: "IMPORT / 作品导入",
    role: "Asset",
    description: "从已有项目或作品库复用镜头、角色和资源。",
    icon: Clock,
    width: 280,
    height: 218,
    outputOffset: 109,
  },
};

const modelOptionsByGroup: Record<ModelGroup, ModelOption[]> = {
  image: [
    { id: "gpt-image-1.5", label: "gpt-image-1.5", subtitle: "OpenAI image generation", baseOutputTokens: 1450 },
    { id: "chatgpt-image-latest", label: "chatgpt-image-latest", subtitle: "ChatGPT image model", baseOutputTokens: 1320 },
    { id: "gpt-image-1", label: "gpt-image-1", subtitle: "OpenAI image generation", baseOutputTokens: 1280 },
    { id: "gpt-image-1-mini", label: "gpt-image-1-mini", subtitle: "Cost-efficient image model", baseOutputTokens: 860 },
  ],
  video: [
    { id: "seedance-2.0", label: "Seedance 2.0", subtitle: "ByteDance video generation", baseOutputTokens: 7600 },
    { id: "seedance-1.5-pro", label: "Seedance 1.5 Pro", subtitle: "ByteDance pro video model", baseOutputTokens: 8200 },
    { id: "kling-2.1", label: "Kling 2.1", subtitle: "Kuaishou video generation", baseOutputTokens: 7800 },
    { id: "kling-2.1-pro", label: "Kling 2.1 Pro", subtitle: "Kuaishou pro video model", baseOutputTokens: 8800 },
    { id: "hailuo-02", label: "MiniMax Hailuo 02", subtitle: "MiniMax video generation", baseOutputTokens: 7200 },
    { id: "hailuo-video-01", label: "Hailuo Video 01", subtitle: "MiniMax Hailuo video model", baseOutputTokens: 6200 },
  ],
};

const nodePaletteSections: NodePaletteSection[] = [
  {
    label: "添加节点",
    items: [
      { label: "文本", type: "text", icon: TypeIcon },
      { label: "图片", type: "image", icon: ImageIcon },
      { label: "视频", type: "video", icon: Video },
      { label: "3D 世界", type: "world3d", icon: Globe2 },
      { label: "音频", type: "audio", icon: AudioLines },
    ],
  },
  {
    label: "功能节点",
    items: [
      { label: "分镜格子", type: "storyboard", icon: Grid2x2 },
      { label: "AI 应用", type: "aiApp", icon: Layers },
    ],
  },
  {
    label: "添加资源",
    items: [
      { label: "上传", type: "source", icon: Upload },
      { label: "从作品导入", type: "import", icon: Clock },
    ],
  },
];

const nodes = reactive<CanvasNode[]>([
  {
    id: "source-1",
    type: "source",
    x: 72,
    y: 160,
    width: nodeDefinitions.source.width,
    height: nodeDefinitions.source.height,
    status: "Local reference loaded",
    assetName: "avatar_sci_fi.png",
  },
  {
    id: "image-1",
    type: "image",
    x: 430,
    y: 100,
    width: nodeDefinitions.image.width,
    height: nodeDefinitions.image.height,
    status: "READY 100%",
    modelId: "gpt-image-1.5",
  },
  {
    id: "video-1",
    type: "video",
    x: 815,
    y: 120,
    width: nodeDefinitions.video.width,
    height: nodeDefinitions.video.height,
    status: "Waiting for rendered image",
    modelId: "seedance-2.0",
  },
]);

const workflowEdges = ref<WorkflowEdge[]>([
  { id: "edge-source-1-image-1", from: "source-1", to: "image-1" },
  { id: "edge-image-1-video-1", from: "image-1", to: "video-1" },
]);

const selectedNodeId = ref("image-1");
const nodeSequence = ref(2);
const dragState = ref<DragState | null>(null);
const canvasPanState = ref<CanvasPanState | null>(null);
const nodeContextMenu = ref<NodeContextMenuState | null>(null);
const canvasViewport = ref<HTMLElement | null>(null);
const canvasScale = ref(1);
const canvasOffset = reactive<Point>({ x: 0, y: 0 });
const uiScale = ref(1);
const isCompactViewport = ref(false);
const isInspectorOpen = ref(false);
const isNodePaletteOpen = ref(false);
let canvasResizeObserver: ResizeObserver | undefined;

const workspaceNav = [
  { label: "我的项目", count: "18", icon: Folder },
  { label: "模型设置", icon: SlidersHorizontal },
  { label: "导出记录", icon: Download },
];

const hasSavedApiKey = computed(() => apiKeySettings.value.apiKey.trim().length > 0);

const supportNav = computed<SupportNavItem[]>(() => [
  { label: "API Key 设置", meta: hasSavedApiKey.value ? "saved" : "empty", icon: KeyRound, action: "api-key" },
  { label: "关于开源", icon: Box },
]);

const tools = ["IQ", "CUT", "REF", "AUD", "FIX"];

const legend = [
  { label: "图像源", dotClass: "bg-[#3b82f6]" },
  { label: "AI 绘图", dotClass: "bg-[#10b981]" },
  { label: "时间维度扩展", dotClass: "bg-[#14b8a6]" },
];

const motionOptions = [
  { label: "低", value: "low" },
  { label: "中", value: "medium" },
  { label: "高", value: "high" },
];

const renderProgress = computed(() => Math.round((renderSteps.value / 55) * 100));
const cfgProgress = computed(() => Math.round((cfgScale.value / 14) * 100));
const selectedCanvasNode = computed(() => findNode(selectedNodeId.value));
const nodeContextMenuTarget = computed(() => (nodeContextMenu.value ? findNode(nodeContextMenu.value.nodeId) : undefined));
const primaryImageNode = computed(() => findPrimaryNode("image"));
const primaryVideoNode = computed(() => findPrimaryNode("video"));
const selectedNodeModel = computed(() => {
  const node = selectedCanvasNode.value;

  return node ? getNodeModel(node) : undefined;
});
const wires = computed<WireDisplay[]>(() =>
  workflowEdges.value.flatMap((edge) => {
    const from = socketPoint(edge.from, "right");
    const to = socketPoint(edge.to, "left");

    if (!from || !to) {
      return [];
    }

    return [
      {
        id: edge.id,
        path: createWirePath(from, to),
        signal: midpoint(from, to),
        signalClass: getWireSignalClass(edge.to),
      },
    ];
  }),
);
const savedApiKeyHint = computed(() => maskApiKey(apiKeySettings.value.apiKey));
const activeProviderLabel = computed(() => getProviderLabel(apiKeySettings.value.provider));
const activeCameraBodies = computed(() => cameraBodies.filter((camera) => camera.category === selectedCameraCategory.value));
const activeCameraProfile = computed(() => findCameraBody(selectedCameraBodyId.value) ?? activeCameraBodies.value[0] ?? cameraBodies[0]);
const activeLensProfile = computed(() => lensOptions.find((lens) => lens.value === selectedLens.value) ?? lensOptions[0]);
const activeLensEffect = computed(() => lensEffects.find((effect) => effect.value === selectedLensEffect.value) ?? lensEffects[0]);
const cameraSettingSummary = computed(
  () => `${activeCameraProfile.value.label} · ${selectedFocalLength.value} · ${selectedAperture.value}`,
);
const imageStatus = computed(() => primaryImageNode.value?.status ?? "READY 100%");
const videoStatus = computed(() => primaryVideoNode.value?.status ?? "Waiting for rendered image");
const isGeneratingImage = computed(() => (primaryImageNode.value ? isNodeRunning(primaryImageNode.value) : false));
const isGeneratingVideo = computed(() => (primaryVideoNode.value ? isNodeRunning(primaryVideoNode.value) : false));
const isCanvasPanning = computed(() => canvasPanState.value !== null);
const canvasFrameStyle = computed(() => ({
  width: `${Math.round(canvasBounds.width * canvasScale.value)}px`,
  height: `${Math.round(canvasBounds.height * canvasScale.value)}px`,
}));
const canvasPanStyle = computed(() => ({
  transform: `translate(${canvasOffset.x}px, ${canvasOffset.y}px)`,
}));
const canvasStageStyle = computed(() => ({
  width: `${canvasBounds.width}px`,
  height: `${canvasBounds.height}px`,
  transform: `scale(${canvasScale.value})`,
}));
const nodeContextMenuStyle = computed(() => ({
  left: `${nodeContextMenu.value?.x ?? 0}px`,
  top: `${nodeContextMenu.value?.y ?? 0}px`,
}));
const appShellStyle = computed(() => {
  if (isCompactViewport.value) {
    return {};
  }

  return {
    zoom: uiScale.value,
  };
});

onMounted(() => {
  void loadApiKeySettings();

  updateViewportSizing();
  canvasResizeObserver = new ResizeObserver(updateCanvasScale);

  if (canvasViewport.value) {
    canvasResizeObserver.observe(canvasViewport.value);
  }

  window.addEventListener("resize", updateViewportSizing);
  window.addEventListener("keydown", handleWindowKeydown);
});

function getProviderBaseUrl(provider: string) {
  return apiProviderOptions.find((option) => option.value === provider)?.baseUrl ?? defaultApiKeySettings.baseUrl;
}

function getProviderLabel(provider: string) {
  return apiProviderOptions.find((option) => option.value === provider)?.label ?? "自定义";
}

function normalizeApiKeySettings(settings?: Partial<ApiKeySettings> | null): ApiKeySettings {
  const provider = settings?.provider?.trim() || defaultApiKeySettings.provider;
  const baseUrl = (settings?.baseUrl?.trim() || getProviderBaseUrl(provider)).replace(/\/+$/, "");

  return {
    provider,
    baseUrl,
    apiKey: settings?.apiKey?.trim() || "",
  };
}

function readLocalApiKeySettings() {
  try {
    const rawSettings = window.localStorage.getItem(API_KEY_STORAGE_KEY);
    return rawSettings ? normalizeApiKeySettings(JSON.parse(rawSettings) as Partial<ApiKeySettings>) : { ...defaultApiKeySettings };
  } catch {
    return { ...defaultApiKeySettings };
  }
}

function writeLocalApiKeySettings(settings: ApiKeySettings) {
  window.localStorage.setItem(API_KEY_STORAGE_KEY, JSON.stringify(settings));
}

function removeLocalApiKeySettings() {
  window.localStorage.removeItem(API_KEY_STORAGE_KEY);
}

function maskApiKey(apiKey: string) {
  const normalizedKey = apiKey.trim();

  if (!normalizedKey) {
    return "未配置";
  }

  if (normalizedKey.length <= 10) {
    return `${normalizedKey.slice(0, 2)}••••${normalizedKey.slice(-2)}`;
  }

  return `${normalizedKey.slice(0, 6)}••••${normalizedKey.slice(-4)}`;
}

async function loadApiKeySettings() {
  try {
    apiKeySettings.value = normalizeApiKeySettings(await invoke<ApiKeySettings>("load_api_key_settings"));
  } catch {
    apiKeySettings.value = readLocalApiKeySettings();
  }

  apiKeyDraft.value = { ...apiKeySettings.value };
  apiKeyMessage.value = hasSavedApiKey.value
    ? `已加载 ${activeProviderLabel.value} · ${savedApiKeyHint.value}`
    : "尚未配置 API Key";
}

function openSupportItem(action?: SupportNavItem["action"]) {
  if (action === "api-key") {
    openApiKeyPanel();
  }
}

function openApiKeyPanel() {
  apiKeyDraft.value = { ...apiKeySettings.value };
  isApiKeyPanelOpen.value = true;
}

function closeApiKeyPanel() {
  isApiKeyPanelOpen.value = false;
  showApiKey.value = false;
}

function selectApiProvider(provider: ApiProviderValue) {
  const currentBaseUrl = getProviderBaseUrl(apiKeyDraft.value.provider);
  const nextBaseUrl = getProviderBaseUrl(provider);

  apiKeyDraft.value.provider = provider;

  if (!apiKeyDraft.value.baseUrl || apiKeyDraft.value.baseUrl === currentBaseUrl) {
    apiKeyDraft.value.baseUrl = nextBaseUrl;
  }
}

async function saveApiKeySettings() {
  const settings = normalizeApiKeySettings(apiKeyDraft.value);

  if (!settings.apiKey) {
    apiKeyMessage.value = "请输入 API Key 后再保存";
    hostStatus.value = "API Key required";
    return;
  }

  isSavingApiKey.value = true;

  try {
    apiKeySettings.value = normalizeApiKeySettings(
      await invoke<ApiKeySettings>("save_api_key_settings", { settings }),
    );
    removeLocalApiKeySettings();
  } catch {
    apiKeySettings.value = settings;
    writeLocalApiKeySettings(settings);
  } finally {
    apiKeyDraft.value = { ...apiKeySettings.value };
    isSavingApiKey.value = false;
    apiKeyMessage.value = `已保存 ${activeProviderLabel.value} · ${savedApiKeyHint.value}`;
    hostStatus.value = `API Key ready: ${activeProviderLabel.value}`;
  }
}

async function clearApiKeySettings() {
  isClearingApiKey.value = true;

  try {
    await invoke("clear_api_key_settings");
  } catch {
    // Web preview does not have Tauri commands, so local storage is the fallback path.
  } finally {
    removeLocalApiKeySettings();
    apiKeySettings.value = { ...defaultApiKeySettings };
    apiKeyDraft.value = { ...defaultApiKeySettings };
    apiKeyMessage.value = "API Key 已清除";
    hostStatus.value = "API Key required";
    isClearingApiKey.value = false;
  }
}

function ensureApiKeyConfigured() {
  if (hasSavedApiKey.value) {
    return true;
  }

  apiKeyMessage.value = "请先保存 API Key，再启动 AI 生成";
  hostStatus.value = "API Key required";
  openApiKeyPanel();
  return false;
}

function clamp(value: number, min: number, max: number) {
  return Math.min(Math.max(value, min), max);
}

function updateUiScale() {
  isCompactViewport.value = window.innerWidth < 760;

  if (isCompactViewport.value) {
    uiScale.value = 1;
    return;
  }

  const widthProgress =
    (window.innerWidth - MIN_UI_SCALE_WIDTH) / (TARGET_UI_SCALE_WIDTH - MIN_UI_SCALE_WIDTH);
  const heightProgress =
    (window.innerHeight - MIN_UI_SCALE_HEIGHT) / (TARGET_UI_SCALE_HEIGHT - MIN_UI_SCALE_HEIGHT);
  const scaleProgress = clamp(Math.min(widthProgress, heightProgress), 0, 1);

  uiScale.value = Number((1 + (MAX_UI_SCALE - 1) * scaleProgress).toFixed(3));
}

function updateViewportSizing() {
  updateUiScale();
  updateCanvasScale();
}

function updateCanvasScale() {
  const viewport = canvasViewport.value;

  if (!viewport) {
    return;
  }

  if (viewport.clientWidth < 720) {
    canvasScale.value = 1;
    return;
  }

  const availableWidth = Math.max(viewport.clientWidth - CANVAS_VIEWPORT_GUTTER, 1);
  const nextScale = Math.min(1, availableWidth / canvasBounds.width);

  canvasScale.value = Number(clamp(nextScale, MIN_CANVAS_SCALE, 1).toFixed(3));
}

function cleanupDragListeners() {
  window.removeEventListener("pointermove", moveDraggedNode);
  window.removeEventListener("pointerup", stopDrag);
  window.removeEventListener("pointercancel", stopDrag);
}

function cleanupCanvasPanListeners() {
  window.removeEventListener("pointermove", moveCanvasPan);
  window.removeEventListener("pointerup", stopCanvasPan);
  window.removeEventListener("pointercancel", stopCanvasPan);
}

function createWirePath(from: Point, to: Point) {
  const direction = to.x >= from.x ? 1 : -1;
  const handle = Math.max(80, Math.abs(to.x - from.x) * 0.45);

  return `M ${from.x} ${from.y} C ${from.x + handle * direction} ${from.y} ${to.x - handle * direction} ${to.y} ${to.x} ${to.y}`;
}

function isInteractiveTarget(target: EventTarget | null) {
  return target instanceof Element && target.closest("button, input, textarea, select, label, a") !== null;
}

function isCanvasPanBlockedTarget(target: EventTarget | null) {
  return target instanceof Element && target.closest("[data-canvas-pan-block]") !== null;
}

function getCanvasPanLimit(axis: "x" | "y") {
  const viewport = canvasViewport.value;

  if (!viewport) {
    return axis === "x" ? 360 : 260;
  }

  return axis === "x"
    ? Math.max(180, Math.round(viewport.clientWidth * 0.48))
    : Math.max(140, Math.round(viewport.clientHeight * 0.42));
}

function midpoint(from: Point, to: Point) {
  return {
    x: Math.round((from.x + to.x) / 2),
    y: Math.round((from.y + to.y) / 2),
  };
}

function findCameraBody(id: string) {
  return cameraBodies.find((camera) => camera.id === id);
}

function openCameraPanel() {
  isCameraPanelOpen.value = true;
}

function closeCameraPanel() {
  isCameraPanelOpen.value = false;
}

function selectCameraCategory(category: CameraCategory) {
  selectedCameraCategory.value = category;
  selectedCameraBodyId.value = cameraBodies.find((camera) => camera.category === category)?.id ?? selectedCameraBodyId.value;
}

function selectCameraPreset(event: Event) {
  const presetValue = (event.target as HTMLSelectElement).value;
  const preset = cameraPresets.find((option) => option.value === presetValue);

  selectedCameraPreset.value = presetValue;

  if (!preset) {
    return;
  }

  selectedLens.value = preset.lens;
  selectedFocalLength.value = preset.focalLength;
  selectedAperture.value = preset.aperture;
  selectedLensEffect.value = preset.effect;
}

function createCameraPromptCue() {
  return [
    activeCameraProfile.value.promptCue,
    `${selectedLens.value} ${selectedFocalLength.value} at ${selectedAperture.value}`,
    activeLensProfile.value.promptCue,
    activeLensEffect.value.promptCue,
  ].join(", ");
}

function mergeCameraCueIntoPrompt(currentPrompt: string, cue: string) {
  const promptLines = currentPrompt
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line && !line.startsWith(CAMERA_PROMPT_PREFIX));

  return [...promptLines, `${CAMERA_PROMPT_PREFIX} ${cue}`].join("\n");
}

function applyCameraControl() {
  prompt.value = mergeCameraCueIntoPrompt(prompt.value, createCameraPromptCue());

  if (primaryImageNode.value) {
    primaryImageNode.value.status = "CAMERA SET";
    selectedNodeId.value = primaryImageNode.value.id;
  }

  hostStatus.value = `Camera applied: ${activeCameraProfile.value.label}`;
  closeCameraPanel();
}

function findNode(id: string) {
  return nodes.find((node) => node.id === id);
}

function findPrimaryNode(type: NodeType) {
  return nodes.find((node) => node.id === `${type}-1`) ?? nodes.find((node) => node.type === type);
}

function resolveNodeReference(reference: string) {
  return findNode(reference) ?? nodes.find((node) => node.type === reference);
}

function isCanvasNode(value: unknown): value is CanvasNode {
  return typeof value === "object" && value !== null && "id" in value && "status" in value;
}

function getDefaultModelId(type: NodeType) {
  const group = nodeDefinitions[type].modelGroup;

  return group ? modelOptionsByGroup[group][0]?.id : undefined;
}

function getDefaultNodeStatus(type: NodeType) {
  const statusByType: Record<NodeType, string> = {
    source: "Local reference loaded",
    text: "Draft ready",
    image: "READY 100%",
    video: "Waiting for rendered image",
    world3d: "Scene space ready",
    audio: "No track attached",
    storyboard: "Ready to arrange",
    aiApp: "App not configured",
    import: "Choose source work",
  };

  return statusByType[type];
}

function getDefaultAssetName(type: NodeType, sequence: number) {
  if (type === "source") {
    return `reference_${sequence}.png`;
  }

  if (type === "import") {
    return `work_${sequence}.linglux`;
  }

  return undefined;
}

function getModelOptionsForNode(node?: CanvasNode) {
  const group = node ? nodeDefinitions[node.type].modelGroup : undefined;

  return group ? modelOptionsByGroup[group] : [];
}

function getNodeModel(node: CanvasNode) {
  const options = getModelOptionsForNode(node);
  const model = options.find((option) => option.id === node.modelId);

  return model ?? options[0];
}

function estimateTextTokens(text: string) {
  const normalizedText = text.trim();

  if (!normalizedText) {
    return 0;
  }

  const cjkCharacters = normalizedText.match(/[\u3400-\u9fff\u3000-\u303f\uff00-\uffef]/g)?.length ?? 0;
  const nonCjkCharacters = normalizedText.length - cjkCharacters;

  return Math.ceil(cjkCharacters * 1.15 + nonCjkCharacters / 4);
}

function formatTokenEstimate(tokens: number) {
  if (tokens >= 1000) {
    return `预计 ≈ ${(tokens / 1000).toFixed(1)}k tokens`;
  }

  return `预计 ≈ ${tokens} tokens`;
}

function getMotionTokenMultiplier() {
  if (selectedMotion.value === "low") {
    return 0.85;
  }

  if (selectedMotion.value === "high") {
    return 1.15;
  }

  return 1;
}

function estimateNodeTokens(node: CanvasNode) {
  const model = getNodeModel(node);

  if (!model) {
    return 0;
  }

  const promptTokens = estimateTextTokens(prompt.value);
  const negativePromptTokens = estimateTextTokens(negativePrompt.value);

  if (node.type === "video") {
    const controlTokens = 180;
    return Math.round((model.baseOutputTokens + promptTokens + negativePromptTokens + controlTokens) * getMotionTokenMultiplier());
  }

  if (node.type === "image") {
    const renderControlTokens = Math.round(40 + renderSteps.value * 1.6 + cfgScale.value * 8);
    return Math.round(model.baseOutputTokens + promptTokens + negativePromptTokens + renderControlTokens);
  }

  return promptTokens;
}

function getNodeTokenEstimateLabel(node: CanvasNode) {
  return formatTokenEstimate(estimateNodeTokens(node));
}

function getWireSignalClass(nodeId: string) {
  const node = findNode(nodeId);

  return node?.type === "video" ? "fill-[#14b8a6]" : "fill-[#10b981]";
}

function hasInput(node: CanvasNode) {
  return nodeDefinitions[node.type].inputOffset !== undefined;
}

function hasOutput(node: CanvasNode) {
  return nodeDefinitions[node.type].outputOffset !== undefined;
}

function createNode(type: NodeType, x: number, y: number): CanvasNode {
  const definition = nodeDefinitions[type];
  const sequence = nodeSequence.value;
  nodeSequence.value += 1;

  return {
    id: `${type}-${sequence}`,
    type,
    x,
    y,
    width: definition.width,
    height: definition.height,
    status: getDefaultNodeStatus(type),
    assetName: getDefaultAssetName(type, sequence),
    modelId: getDefaultModelId(type),
  };
}

function doNodeRectsOverlap(candidate: Pick<CanvasNode, "x" | "y" | "width" | "height">, node: CanvasNode) {
  const gap = 34;

  return (
    candidate.x < node.x + node.width + gap &&
    candidate.x + candidate.width + gap > node.x &&
    candidate.y < node.y + node.height + gap &&
    candidate.y + candidate.height + gap > node.y
  );
}

function isNodePositionFree(type: NodeType, x: number, y: number) {
  const definition = nodeDefinitions[type];
  const candidate = {
    x,
    y,
    width: definition.width,
    height: definition.height,
  };
  const isInsideCanvas =
    x >= canvasBounds.padding &&
    y >= canvasBounds.topPadding &&
    x + definition.width <= canvasBounds.width - canvasBounds.padding &&
    y + definition.height <= canvasBounds.height - canvasBounds.padding;

  return isInsideCanvas && nodes.every((node) => !doNodeRectsOverlap(candidate, node));
}

function getNewNodePosition(type: NodeType, anchorNode?: CanvasNode) {
  const definition = nodeDefinitions[type];
  const cascade = (nodes.length % 4) * 22;
  const candidates: Point[] = [];

  if (anchorNode) {
    candidates.push({ x: anchorNode.x + anchorNode.width + 86, y: anchorNode.y });
  }

  for (const y of [100, 480, 700, 920]) {
    for (const x of [72, 430, 815]) {
      candidates.push({ x, y });
    }
  }

  if (anchorNode) {
    candidates.push({ x: anchorNode.x, y: anchorNode.y + anchorNode.height + 70 });
  }

  candidates.push({ x: 92 + cascade, y: 120 + cascade });

  const freeCandidate = candidates.find((candidate) => isNodePositionFree(type, candidate.x, candidate.y));
  const fallback = freeCandidate ?? candidates[candidates.length - 1];

  return {
    x: clamp(Math.round(fallback.x), canvasBounds.padding, canvasBounds.width - definition.width - canvasBounds.padding),
    y: clamp(Math.round(fallback.y), canvasBounds.topPadding, canvasBounds.height - definition.height - canvasBounds.padding),
  };
}

function connectNodes(from: CanvasNode, to: CanvasNode) {
  if (!hasOutput(from) || !hasInput(to)) {
    return;
  }

  const edgeId = `edge-${from.id}-${to.id}`;

  if (workflowEdges.value.some((edge) => edge.id === edgeId)) {
    return;
  }

  workflowEdges.value.push({ id: edgeId, from: from.id, to: to.id });
}

function connectNewNode(anchorNode: CanvasNode | undefined, node: CanvasNode) {
  if (!anchorNode) {
    return;
  }

  if (hasOutput(anchorNode) && hasInput(node)) {
    connectNodes(anchorNode, node);
    return;
  }

  if (hasOutput(node) && hasInput(anchorNode)) {
    connectNodes(node, anchorNode);
  }
}

function addWorkflowNode(type: NodeType) {
  const anchorNode = selectedCanvasNode.value;
  const position = getNewNodePosition(type, anchorNode);
  const node = createNode(type, position.x, position.y);

  nodes.push(node);
  connectNewNode(anchorNode, node);
  selectedNodeId.value = node.id;
  isInspectorOpen.value = true;
  closeNodePalette();
  hostStatus.value = `Added ${nodeDefinitions[node.type].label} node`;
}

function closeInspector() {
  isInspectorOpen.value = false;
}

function toggleNodePalette() {
  isNodePaletteOpen.value = !isNodePaletteOpen.value;
  closeNodeContextMenu();
}

function closeNodePalette() {
  isNodePaletteOpen.value = false;
}

function closeNodeContextMenu() {
  nodeContextMenu.value = null;
}

function closeFloatingPanels() {
  closeNodeContextMenu();
  closeNodePalette();
}

function handleWindowKeydown(event: KeyboardEvent) {
  if (event.key === "Escape") {
    closeFloatingPanels();
  }
}

function openNodeContextMenu(id: string, event: MouseEvent) {
  if (isInteractiveTarget(event.target)) {
    closeNodeContextMenu();
    return;
  }

  const node = findNode(id);

  if (!node) {
    closeNodeContextMenu();
    return;
  }

  event.preventDefault();
  event.stopPropagation();
  stopDrag();
  stopCanvasPan();
  closeNodePalette();

  const maxX = Math.max(
    NODE_CONTEXT_MENU_MARGIN,
    window.innerWidth / uiScale.value - NODE_CONTEXT_MENU_WIDTH - NODE_CONTEXT_MENU_MARGIN,
  );
  const maxY = Math.max(
    NODE_CONTEXT_MENU_MARGIN,
    window.innerHeight / uiScale.value - NODE_CONTEXT_MENU_HEIGHT - NODE_CONTEXT_MENU_MARGIN,
  );

  selectedNodeId.value = node.id;
  isInspectorOpen.value = true;
  nodeContextMenu.value = {
    nodeId: node.id,
    x: clamp(event.clientX / uiScale.value, NODE_CONTEXT_MENU_MARGIN, maxX),
    y: clamp(event.clientY / uiScale.value, NODE_CONTEXT_MENU_MARGIN, maxY),
  };
}

function deleteContextMenuNode() {
  const nodeId = nodeContextMenu.value?.nodeId;

  if (nodeId) {
    deleteWorkflowNode(nodeId);
  }
}

function deleteWorkflowNode(id: string) {
  const nodeIndex = nodes.findIndex((node) => node.id === id);

  if (nodeIndex === -1) {
    closeNodeContextMenu();
    return;
  }

  const deletedNode = nodes[nodeIndex];
  nodes.splice(nodeIndex, 1);
  workflowEdges.value = workflowEdges.value.filter((edge) => edge.from !== id && edge.to !== id);

  if (dragState.value?.id === id) {
    stopDrag();
  }

  if (selectedNodeId.value === id) {
    const nextSelectedNode = nodes[nodeIndex] ?? nodes[nodeIndex - 1] ?? nodes[0];
    selectedNodeId.value = nextSelectedNode?.id ?? "";
    isInspectorOpen.value = Boolean(nextSelectedNode);
  }

  closeNodeContextMenu();
  hostStatus.value = `Deleted ${nodeDefinitions[deletedNode.type].label} node`;
}

function nodeBorderClass(node: CanvasNode) {
  if (node.type === "image") {
    return "border-[1.5px] border-[#10b981] shadow-[0_0_20px_rgb(16_185_129/0.18)]";
  }

  return "border border-[#222228]";
}

function nodeDotClass(node: CanvasNode) {
  if (node.type === "source") {
    return "bg-[#3b82f6] shadow-[0_0_14px_rgb(59_130_246/0.55)]";
  }

  if (node.type === "video") {
    return "bg-[#14b8a6] shadow-[0_0_14px_rgb(20_184_166/0.55)]";
  }

  return "bg-[#10b981] shadow-[0_0_14px_rgb(16_185_129/0.6)]";
}

function nodeRoleClass(node: CanvasNode) {
  if (node.type === "source") {
    return "border-[#3b82f6] bg-[#0b1b36] text-[#3b82f6]";
  }

  if (node.type === "video") {
    return "border-[#14b8a6] bg-[#0a2425] text-[#14b8a6]";
  }

  return "border-[#10b981] bg-[#0f2a20] text-[#10b981]";
}

function nodeStyle(nodeReference: CanvasNode | string) {
  const node = typeof nodeReference === "string" ? resolveNodeReference(nodeReference) : nodeReference;

  if (!node) {
    return {};
  }

  const isSelected = selectedNodeId.value === node.id;

  return {
    left: `${node.x}px`,
    top: `${node.y}px`,
    width: `${node.width}px`,
    minHeight: `${node.height}px`,
    zIndex: isSelected ? 4 : 2,
  };
}

function socketPoint(id: string, side: NodeSide): Point | undefined {
  const node = findNode(id);

  if (!node) {
    return undefined;
  }

  const definition = nodeDefinitions[node.type];
  const offset = side === "right" ? definition.outputOffset : definition.inputOffset;

  if (offset === undefined) {
    return undefined;
  }

  return {
    x: side === "right" ? node.x + node.width : node.x,
    y: node.y + offset,
  };
}

function startDrag(id: string, event: PointerEvent) {
  const node = resolveNodeReference(id);

  if (!node) {
    return;
  }

  selectedNodeId.value = node.id;
  isInspectorOpen.value = true;

  if (!event.isPrimary || event.button !== 0 || isInteractiveTarget(event.target)) {
    return;
  }

  dragState.value = {
    id: node.id,
    pointerId: event.pointerId,
    startX: event.clientX,
    startY: event.clientY,
    originX: node.x,
    originY: node.y,
  };

  event.preventDefault();
  cleanupDragListeners();
  window.addEventListener("pointermove", moveDraggedNode);
  window.addEventListener("pointerup", stopDrag);
  window.addEventListener("pointercancel", stopDrag);
}

function moveDraggedNode(event: PointerEvent) {
  const drag = dragState.value;

  if (!drag || event.pointerId !== drag.pointerId) {
    return;
  }

  const node = findNode(drag.id);

  if (!node) {
    return;
  }

  const nextX = drag.originX + (event.clientX - drag.startX) / (canvasScale.value * uiScale.value);
  const nextY = drag.originY + (event.clientY - drag.startY) / (canvasScale.value * uiScale.value);

  node.x = clamp(Math.round(nextX), canvasBounds.padding, canvasBounds.width - node.width - canvasBounds.padding);
  node.y = clamp(Math.round(nextY), canvasBounds.topPadding, canvasBounds.height - node.height - canvasBounds.padding);
}

function stopDrag(event?: PointerEvent) {
  const drag = dragState.value;

  if (event && drag && event.pointerId !== drag.pointerId) {
    return;
  }

  dragState.value = null;
  cleanupDragListeners();
}

function startCanvasPan(event: PointerEvent) {
  if (!event.isPrimary || event.button !== 0 || isInteractiveTarget(event.target) || isCanvasPanBlockedTarget(event.target)) {
    return;
  }

  canvasPanState.value = {
    pointerId: event.pointerId,
    startX: event.clientX,
    startY: event.clientY,
    originX: canvasOffset.x,
    originY: canvasOffset.y,
  };

  event.preventDefault();
  cleanupCanvasPanListeners();
  window.addEventListener("pointermove", moveCanvasPan);
  window.addEventListener("pointerup", stopCanvasPan);
  window.addEventListener("pointercancel", stopCanvasPan);
}

function moveCanvasPan(event: PointerEvent) {
  const pan = canvasPanState.value;

  if (!pan || event.pointerId !== pan.pointerId) {
    return;
  }

  const xLimit = getCanvasPanLimit("x");
  const yLimit = getCanvasPanLimit("y");

  canvasOffset.x = Math.round(clamp(pan.originX + (event.clientX - pan.startX) / uiScale.value, -xLimit, xLimit));
  canvasOffset.y = Math.round(clamp(pan.originY + (event.clientY - pan.startY) / uiScale.value, -yLimit, yLimit));
}

function stopCanvasPan(event?: PointerEvent) {
  const pan = canvasPanState.value;

  if (event && pan && event.pointerId !== pan.pointerId) {
    return;
  }

  canvasPanState.value = null;
  cleanupCanvasPanListeners();
}

function isNodeRunning(node: CanvasNode) {
  return node.status === "RENDERING" || node.status === "Queued for diffusion";
}

async function generateImage(node?: CanvasNode | Event) {
  const targetNode = isCanvasNode(node) ? node : primaryImageNode.value;

  if (!targetNode) {
    return;
  }

  if (!ensureApiKeyConfigured()) {
    return;
  }

  targetNode.status = "RENDERING";
  hostStatus.value = `Rendering with ${getNodeModel(targetNode)?.label ?? "selected model"}`;

  try {
    await invoke<string>("create_video_plan", { prompt: prompt.value });
    targetNode.status = "READY 100%";
    hostStatus.value = "Image node refreshed";
  } catch {
    targetNode.status = "READY 100%";
    hostStatus.value = "Preview bridge fallback";
  }
}

function generateVideo(node?: CanvasNode | Event) {
  const targetNode = isCanvasNode(node) ? node : primaryVideoNode.value;

  if (!targetNode) {
    return;
  }

  if (!ensureApiKeyConfigured()) {
    return;
  }

  targetNode.status = "Queued for diffusion";
  hostStatus.value = `Queued with ${getNodeModel(targetNode)?.label ?? "selected model"}`;

  window.setTimeout(() => {
    targetNode.status = "Ready to render";
  }, 650);
}

onUnmounted(() => {
  stopDrag();
  stopCanvasPan();
  canvasResizeObserver?.disconnect();
  window.removeEventListener("resize", updateViewportSizing);
  window.removeEventListener("keydown", handleWindowKeydown);
});
</script>

<template>
  <main
    class="grid h-dvh grid-cols-[236px_minmax(0,1fr)] overflow-hidden bg-[#070708] text-[#d1d5db] max-[760px]:h-auto max-[760px]:min-h-dvh max-[760px]:grid-cols-1"
    :style="appShellStyle"
    @pointerdown="closeFloatingPanels"
  >
    <aside class="flex min-w-0 flex-col border-r border-[#222228] bg-[#0e0e11] max-[760px]:border-r-0 max-[760px]:border-b">
      <header class="grid h-[72px] grid-cols-[34px_1fr] items-center gap-3 border-b border-[#222228] px-5">
        <img :src="lingluxLogo" alt="" class="size-[34px] rounded-[9px] object-contain shadow-[0_0_26px_rgb(16_185_129/0.24)]" />
        <div class="min-w-0">
          <h1 class="truncate text-[14px] font-bold leading-[18px] text-white">灵帧AI</h1>
          <p class="truncate text-[9px] font-semibold leading-3 text-[#6b7280]">LINGLUX AI</p>
        </div>
      </header>

      <button class="mx-3 mb-6 mt-[18px] inline-flex h-10 items-center gap-2.5 rounded-lg border-0 bg-[#1a1a22] px-3.5 text-[13px] font-bold text-[#10b981] hover:bg-[#1e2424] max-[760px]:mb-4" type="button">
        <Plus :size="15" />
        新建工作流
      </button>

      <nav class="mx-3 mb-[22px] grid gap-1.5" aria-label="Workspace">
        <p class="mb-2.5 ml-3 text-[10px] font-extrabold leading-3 text-[#4b5563]">工作区</p>
        <button v-for="item in workspaceNav" :key="item.label" class="grid h-[34px] grid-cols-[24px_1fr_auto] items-center gap-1.5 rounded-lg px-3 text-left text-[12px] text-[#9ca3af] hover:bg-[#15151a]" type="button">
          <span class="grid size-4 place-items-center rounded-[3px] border border-[#33333a] bg-[#1d1d22] text-[#6b7280]">
            <component :is="item.icon" :size="14" />
          </span>
          <span class="truncate">{{ item.label }}</span>
          <small v-if="item.count" class="inline-flex min-h-[17px] min-w-[26px] items-center justify-center rounded-full bg-[#1f1f24] text-[10px] text-[#6b7280]">
            {{ item.count }}
          </small>
        </button>
      </nav>

      <nav class="mx-3 mb-[22px] grid gap-1.5" aria-label="Settings and support">
        <p class="mb-2.5 ml-3 text-[10px] font-extrabold leading-3 text-[#4b5563]">设置与支持</p>
        <button
          v-for="item in supportNav"
          :key="item.label"
          class="grid h-[34px] grid-cols-[24px_1fr_auto] items-center gap-1.5 rounded-lg px-3 text-left text-[12px] text-[#9ca3af] hover:bg-[#15151a]"
          :class="isApiKeyPanelOpen && item.action === 'api-key' ? 'bg-[#15151a] text-[#d1d5db]' : ''"
          type="button"
          @click="openSupportItem(item.action)"
        >
          <span class="grid size-4 place-items-center text-[#10b981]">
            <component :is="item.icon" :size="14" />
          </span>
          <span class="truncate">{{ item.label }}</span>
          <i
            v-if="item.meta"
            class="size-2 rounded-full"
            :class="item.meta === 'saved' ? 'bg-[#10b981] shadow-[0_0_14px_rgb(16_185_129/0.48)]' : 'bg-[#4b5563]'"
            aria-hidden="true"
          ></i>
        </button>
      </nav>

      <footer class="mt-auto grid gap-3.5 border-t border-[#222228] bg-[#070708] px-[18px] pb-3.5 pt-5 max-[760px]:mt-1.5">
        <div class="flex items-center gap-2 text-[11px] text-[#9ca3af]">
          <span class="size-[7px] rounded-full bg-[#10b981] shadow-[0_0_14px_rgb(16_185_129/0.6)]"></span>
          <span class="truncate">{{ hostStatus }}</span>
        </div>
        <div class="flex items-center justify-between gap-2 text-[10px] text-[#9ca3af]">
          <span>Target: macOS/Win</span>
          <span>v0.1.0</span>
        </div>
      </footer>
    </aside>

    <section
      ref="canvasViewport"
      class="canvas-grid relative min-w-0 overflow-auto bg-[#070708] [scrollbar-color:#2a2a32_#0b0b0d] max-[760px]:h-[760px]"
      :class="isCanvasPanning ? 'cursor-grabbing' : 'cursor-grab'"
      aria-label="AI workflow canvas"
      @pointerdown="startCanvasPan"
    >
      <div data-canvas-pan-block class="sticky left-1/2 top-5 z-10 inline-flex min-h-11 -translate-x-1/2 flex-wrap items-center gap-2 rounded-full border border-[#222228] bg-[#101014]/90 px-3.5 py-1.5 shadow-[0_10px_30px_rgb(0_0_0/0.35)] backdrop-blur max-[760px]:left-4 max-[760px]:translate-x-0" aria-label="Floating tools">
        <span class="text-[11px] font-semibold text-[#6b7280]">工具箱</span>
        <b class="h-4 w-px bg-[#222228]" aria-hidden="true"></b>
        <button v-for="tool in tools" :key="tool" type="button" :title="tool" class="grid size-7 place-items-center rounded-full border border-[#222228] bg-[#141418] text-[10px] font-extrabold text-[#9ca3af] hover:text-[#10b981]">
          {{ tool }}
        </button>
        <b class="h-4 w-px bg-[#222228]" aria-hidden="true"></b>
        <button
          class="grid size-7 place-items-center rounded-full border border-[#222228] bg-[#141418] text-[#10b981] hover:bg-[#10231d]"
          type="button"
          title="摄影机控制"
          aria-label="打开摄影机控制"
          @click="openCameraPanel"
        >
          <Camera :size="14" />
        </button>
        <button class="grid size-7 place-items-center rounded-full border border-[#222228] bg-[#141418] text-[10px] font-extrabold text-[#10b981]" type="button" title="Export">
          DL
        </button>
        <b class="h-4 w-px bg-[#222228]" aria-hidden="true"></b>
        <div class="relative" @pointerdown.stop>
          <button
            class="inline-flex h-7 items-center gap-1.5 whitespace-nowrap rounded-full border border-[#10b981] bg-[#10231d] px-3 text-[10px] font-extrabold text-[#10b981] outline-none hover:bg-[#123026] focus-visible:ring-2 focus-visible:ring-[#10b981]/35"
            type="button"
            :aria-expanded="isNodePaletteOpen"
            aria-haspopup="menu"
            aria-label="打开添加节点菜单"
            @click="toggleNodePalette"
          >
            <Plus :size="13" />
            添加节点
          </button>

          <Transition name="node-palette">
            <div
              v-if="isNodePaletteOpen"
              class="absolute right-0 top-10 z-30 w-[300px] overflow-hidden rounded-xl border border-[#303034] bg-[#1d1d1f] px-[22px] py-2.5 shadow-[0_22px_60px_rgb(0_0_0/0.58)] max-[760px]:left-0 max-[760px]:right-auto"
              role="menu"
              aria-label="添加节点"
              @contextmenu.prevent
            >
              <section v-for="section in nodePaletteSections" :key="section.label" class="py-2">
                <h2 class="mb-1 text-[11px] font-medium leading-5 text-[#8a8a8f]">{{ section.label }}</h2>
                <button
                  v-for="item in section.items"
                  :key="item.type"
                  class="grid h-14 w-full grid-cols-[40px_1fr] items-center gap-3 rounded-lg text-left text-[14px] font-semibold text-[#f4f4f5] outline-none hover:bg-[#26262a] focus-visible:bg-[#26262a] focus-visible:ring-2 focus-visible:ring-[#10b981]/35"
                  type="button"
                  role="menuitem"
                  @click="addWorkflowNode(item.type)"
                >
                  <span class="grid size-10 place-items-center rounded-[10px] bg-[#2b2b2e] text-white shadow-[inset_0_1px_0_rgb(255_255_255/0.04)]">
                    <component :is="item.icon" :size="20" :stroke-width="2.1" />
                  </span>
                  <span class="truncate">{{ item.label }}</span>
                </button>
              </section>
            </div>
          </Transition>
        </div>
      </div>

      <div class="relative mx-auto mt-4" :style="canvasFrameStyle">
        <div class="absolute left-0 top-0 will-change-transform" :style="canvasPanStyle">
        <div class="absolute left-0 top-0 origin-top-left" :style="canvasStageStyle">
        <svg
          class="pointer-events-none absolute left-0 top-0 z-[1]"
          :style="{ height: `${canvasBounds.height}px`, width: `${canvasBounds.width}px` }"
          :viewBox="`0 0 ${canvasBounds.width} ${canvasBounds.height}`"
          aria-hidden="true"
        >
          <defs>
            <linearGradient id="wire-a" x1="0" x2="1" y1="0" y2="0">
              <stop offset="0%" stop-color="#10b981" stop-opacity="0" />
              <stop offset="48%" stop-color="#10b981" />
              <stop offset="100%" stop-color="#14b8a6" />
            </linearGradient>
          </defs>
          <template v-for="wire in wires" :key="wire.id">
            <path class="fill-none stroke-[url(#wire-a)] stroke-2 [filter:drop-shadow(0_0_5px_rgb(16_185_129/0.5))]" stroke-linecap="round" :d="wire.path" />
            <circle class="[filter:drop-shadow(0_0_8px_rgb(16_185_129/0.7))]" :class="wire.signalClass" :cx="wire.signal.x" :cy="wire.signal.y" r="4" />
          </template>
        </svg>

        <article
          v-for="node in nodes"
          :key="node.id"
          data-canvas-pan-block
          class="absolute flex touch-none flex-col overflow-visible rounded-xl bg-[#121216] cursor-default"
          :class="[nodeBorderClass(node), selectedNodeId === node.id ? 'ring-1 ring-[#10b981]/60' : '']"
          :style="nodeStyle(node)"
          :aria-label="`${nodeDefinitions[node.type].label} node`"
          @pointerdown="startDrag(node.id, $event)"
          @contextmenu="openNodeContextMenu(node.id, $event)"
        >
          <header class="grid h-[38px] cursor-grab select-none grid-cols-[auto_1fr_auto] items-center gap-2 border-b border-[#222228] px-4 active:cursor-grabbing">
            <span class="size-[7px] rounded-full" :class="nodeDotClass(node)"></span>
            <strong class="truncate text-[11px] font-semibold leading-[15px] text-[#d1d5db]">{{ nodeDefinitions[node.type].title }}</strong>
            <em class="inline-flex h-[18px] min-w-[58px] items-center justify-center rounded-[5px] border text-[9px] not-italic leading-3" :class="nodeRoleClass(node)">
              {{ nodeDefinitions[node.type].role }}
            </em>
          </header>

          <template v-if="node.type === 'source'">
            <div class="neural-preview relative mx-4 mt-[19px] h-32 overflow-hidden rounded-lg border border-[#262630] px-3.5 pb-3 pt-[45px]">
              <div class="absolute inset-0 bg-black/25"></div>
              <span class="absolute bottom-2.5 left-2.5 z-[1] text-[9px] font-extrabold text-[#cffcf0]">NEURAL FRAME</span>
              <strong class="relative z-[1] block truncate text-[12px] text-[#e0f2fe]">{{ node.assetName ?? "reference.png" }}</strong>
              <small class="relative z-[1] mt-1 block truncate text-[10px] text-[#93c5fd]">{{ node.status }}</small>
            </div>
            <div class="mx-4 mt-4 grid h-[30px] grid-cols-[1fr_auto] items-center gap-2 rounded-[5px] border border-[#222228] bg-[#0e0e11] px-3">
              <span class="truncate text-[10px] font-medium text-[#6b7280]">{{ node.assetName ?? "reference.png" }}</span>
              <small class="text-[10px] text-[#6b7280]">Local</small>
            </div>
            <span class="socket right-[-7px] top-[119px]"></span>
          </template>

          <template v-else-if="node.type === 'image'">
            <div class="neural-preview relative mx-4 mb-4 mt-5 h-40 overflow-hidden rounded-lg border border-[#262630]">
              <span class="absolute bottom-2.5 left-2.5 z-[1] text-[9px] font-extrabold text-[#cffcf0]">NEURAL FRAME</span>
              <span class="absolute right-4 top-3 z-[1] inline-flex h-[22px] min-w-24 items-center justify-center rounded-full border border-[#115e59] bg-[#042f2e] px-3 text-[9px] font-extrabold text-[#5eead4]">
                {{ node.status }}
              </span>
            </div>

            <label class="mx-4 mb-[7px] block text-[10px] font-extrabold leading-[14px] text-[#6b7280]" :for="`image-prompt-${node.id}`">Prompt 画面提示词</label>
            <textarea :id="`image-prompt-${node.id}`" v-model="prompt" class="mx-4 mb-2.5 block h-[58px] w-[calc(100%_-_32px)] resize-none rounded border border-[#222228] bg-[#15151a] px-[9px] py-[7px] text-[11px] leading-[15px] text-[#d1d5db] outline-none focus:border-[#10b981] focus:ring-2 focus:ring-[#10b981]/20" rows="3"></textarea>

            <div class="mx-4 grid grid-cols-2 gap-3">
              <label class="grid min-w-0 gap-1.5">
                <span class="text-[10px] font-extrabold leading-[14px] text-[#6b7280]">图片比例</span>
                <input class="h-7 w-full rounded border border-[#222228] bg-[#15151a] px-[9px] text-[11px] text-[#d1d5db] outline-none focus:border-[#10b981] focus:ring-2 focus:ring-[#10b981]/20" value="3:4 / 中 / 1k" readonly />
              </label>
              <label class="grid min-w-0 gap-1.5">
                <span class="text-[10px] font-extrabold leading-[14px] text-[#6b7280]">模型选择</span>
                <select v-model="node.modelId" class="h-7 w-full rounded border border-[#222228] bg-[#15151a] px-[9px] text-[11px] text-[#d1d5db] outline-none focus:border-[#10b981] focus:ring-2 focus:ring-[#10b981]/20" aria-label="图像模型选择">
                  <option v-for="model in getModelOptionsForNode(node)" :key="model.id" :value="model.id">
                    {{ model.label }}
                  </option>
                </select>
              </label>
            </div>

            <div class="mx-4 mt-3 grid h-8 grid-cols-[1fr_auto] items-center gap-2 rounded-[5px] border border-[#222228] bg-[#0e0e11] px-3">
              <span class="truncate text-[10px] font-medium text-[#9ca3af]">{{ getNodeModel(node)?.subtitle }}</span>
              <strong class="rounded-full bg-[#22302a] px-2 py-0.5 text-[10px] text-[#86efac]">{{ getNodeTokenEstimateLabel(node) }}</strong>
            </div>

            <button class="mx-4 mt-7 inline-flex min-h-9 w-[calc(100%_-_32px)] items-center justify-center gap-2 rounded-lg border-0 bg-[#10b981] px-3 py-2 text-[12px] font-black text-[#070708] hover:brightness-110 disabled:cursor-wait disabled:opacity-75" type="button" :disabled="isNodeRunning(node)" @click="generateImage(node)">
              <WandSparkles :size="15" />
              {{ isNodeRunning(node) ? "生成中..." : hasSavedApiKey ? "生成 AI 图像" : "设置 API Key" }}
            </button>
            <span class="socket left-[-7px] top-[119px]"></span>
            <span class="socket right-[-7px] top-[139px]"></span>
          </template>

          <template v-else-if="node.type === 'video'">
            <div class="mx-4 mb-4 mt-5 grid h-40 place-items-center overflow-hidden rounded-lg border border-[#23232c] bg-[#0e0e11] text-center text-[#6b7280]">
              <Video :size="24" />
              <strong class="text-[13px]">VIDEO</strong>
              <span class="-mt-[18px] text-[11px]">{{ node.status }}</span>
            </div>

            <label class="mx-4 mb-4 grid gap-1.5">
              <span class="text-[10px] font-extrabold leading-[14px] text-[#6b7280]">视频模型</span>
              <select v-model="node.modelId" class="h-7 w-full rounded border border-[#222228] bg-[#15151a] px-[9px] text-[11px] text-[#d1d5db] outline-none focus:border-[#10b981] focus:ring-2 focus:ring-[#10b981]/20" aria-label="视频模型选择">
                <option v-for="model in getModelOptionsForNode(node)" :key="model.id" :value="model.id">
                  {{ model.label }}
                </option>
              </select>
            </label>

            <p class="mx-4 mb-2 block text-[10px] font-extrabold leading-[14px] text-[#6b7280]">运动幅度强度</p>
            <div class="mx-4 mb-5 grid h-8 grid-cols-3 gap-0.5 rounded-[5px] border border-[#222228] bg-[#15151a] p-0.5" role="group" aria-label="Motion intensity">
              <button
                v-for="option in motionOptions"
                :key="option.value"
                type="button"
                class="rounded text-[10px] font-extrabold"
                :class="selectedMotion === option.value ? 'border border-[#115e59] bg-[#0f2f2e] text-[#14b8a6]' : 'border border-transparent text-[#6b7280]'"
                @click="selectedMotion = option.value"
              >
                {{ option.label }}
              </button>
            </div>

            <div class="mx-4 grid grid-cols-2 gap-3">
              <label class="grid min-w-0 gap-1.5">
                <span class="text-[10px] font-extrabold leading-[14px] text-[#6b7280]">时长选择</span>
                <input class="h-7 w-full rounded border border-[#222228] bg-[#15151a] px-[9px] text-[11px] text-[#d1d5db] outline-none focus:border-[#10b981] focus:ring-2 focus:ring-[#10b981]/20" value="5 秒" readonly />
              </label>
              <label class="grid min-w-0 gap-1.5">
                <span class="text-[10px] font-extrabold leading-[14px] text-[#6b7280]">分辨率</span>
                <input class="h-7 w-full rounded border border-[#222228] bg-[#15151a] px-[9px] text-[11px] text-[#d1d5db] outline-none focus:border-[#10b981] focus:ring-2 focus:ring-[#10b981]/20" value="1080p FHD" readonly />
              </label>
            </div>

            <div class="mx-4 mt-3 grid h-8 grid-cols-[1fr_auto] items-center gap-2 rounded-[5px] border border-[#222228] bg-[#0e0e11] px-3">
              <span class="truncate text-[10px] font-medium text-[#9ca3af]">{{ getNodeModel(node)?.subtitle }}</span>
              <strong class="rounded-full bg-[#1a2b2b] px-2 py-0.5 text-[10px] text-[#5eead4]">{{ getNodeTokenEstimateLabel(node) }}</strong>
            </div>

            <button class="mx-4 mt-7 inline-flex min-h-9 w-[calc(100%_-_32px)] items-center justify-center gap-2 rounded-lg border-0 bg-[#14b8a6] px-3 py-2 text-[12px] font-black text-[#070708] hover:brightness-110 disabled:cursor-wait disabled:opacity-75" type="button" :disabled="isNodeRunning(node)" @click="generateVideo(node)">
              <Sparkles :size="15" />
              {{ isNodeRunning(node) ? "排队中..." : hasSavedApiKey ? "生成 AI 视频" : "设置 API Key" }}
            </button>
            <span class="socket left-[-7px] top-[119px]"></span>
          </template>

          <template v-else>
            <div class="mx-4 mt-5 grid gap-3.5">
              <div class="grid min-h-[88px] grid-cols-[42px_1fr] gap-3 rounded-lg border border-[#23232c] bg-[#0e0e11] p-3">
                <span class="grid size-[42px] place-items-center rounded-[10px] bg-[#202024] text-[#d1d5db]">
                  <component :is="nodeDefinitions[node.type].icon" :size="20" />
                </span>
                <div class="min-w-0">
                  <strong class="block truncate text-[13px] font-bold leading-5 text-[#f4f4f5]">{{ nodeDefinitions[node.type].label }}</strong>
                  <span class="mt-1 line-clamp-2 block text-[10px] leading-[1.45] text-[#8a8a8f]">{{ nodeDefinitions[node.type].description }}</span>
                </div>
              </div>

              <div class="grid h-[34px] grid-cols-[1fr_auto] items-center gap-2 rounded-[5px] border border-[#222228] bg-[#0e0e11] px-3">
                <span class="truncate text-[10px] font-medium text-[#9ca3af]">{{ node.assetName ?? nodeDefinitions[node.type].role }}</span>
                <strong class="rounded-full bg-[#18231f] px-2 py-0.5 text-[9px] text-[#86efac]">{{ node.status }}</strong>
              </div>
            </div>

            <span v-if="hasInput(node)" class="socket left-[-7px]" :style="{ top: `${nodeDefinitions[node.type].inputOffset}px` }"></span>
            <span v-if="hasOutput(node)" class="socket right-[-7px]" :style="{ top: `${nodeDefinitions[node.type].outputOffset}px` }"></span>
          </template>
        </article>

        <aside class="absolute bottom-[26px] left-5 inline-flex h-10 items-center gap-3.5 rounded-lg border border-[#222228] bg-[#0f0f12] px-4 text-[11px] text-[#6b7280]" aria-label="Canvas legend">
          <span v-for="item in legend" :key="item.label" class="inline-flex items-center gap-[7px] whitespace-nowrap">
            <i class="size-[7px] rounded-sm" :class="item.dotClass"></i>
            {{ item.label }}
          </span>
          <b class="text-[#4b5563]" aria-hidden="true">|</b>
          <span class="inline-flex items-center gap-[7px] whitespace-nowrap">
            <MousePointer2 :size="13" />
            节点编排已连接
          </span>
        </aside>
        </div>
        </div>
      </div>
    </section>

    <div
      v-if="nodeContextMenu && nodeContextMenuTarget"
      class="fixed z-50 w-[178px] rounded-lg border border-[#2f2f36] bg-[#15151a] p-1 shadow-[0_18px_48px_rgb(0_0_0/0.48)]"
      :style="nodeContextMenuStyle"
      role="menu"
      :aria-label="`节点菜单：${nodeDefinitions[nodeContextMenuTarget.type].label}`"
      @pointerdown.stop
      @contextmenu.prevent
    >
      <button
        class="flex h-9 w-full items-center gap-2 rounded-md px-2.5 text-left text-[12px] font-bold text-[#fca5a5] hover:bg-[#2a1719] hover:text-[#fecaca]"
        type="button"
        role="menuitem"
        @click="deleteContextMenuNode"
      >
        <Trash2 :size="14" />
        <span class="truncate">删除 {{ nodeDefinitions[nodeContextMenuTarget.type].label }}</span>
      </button>
    </div>

    <Transition name="inspector-slide">
      <aside
        v-if="isInspectorOpen && selectedCanvasNode"
        class="fixed bottom-0 right-0 top-0 z-40 flex w-[min(28vw,420px)] min-w-[320px] max-w-[calc(100vw_-_32px)] flex-col border-l border-[#222228] bg-[#0e0e11] shadow-[0_0_48px_rgb(0_0_0/0.5)] max-[760px]:min-w-0"
        aria-label="Inspector panel"
      >
        <header class="grid h-[58px] grid-cols-[auto_1fr_auto] items-center gap-[11px] border-b border-[#222228] px-[18px] text-[#9ca3af]">
          <Settings :size="15" class="text-[#10b981]" />
          <strong class="truncate text-[11px]">INSPECTOR / 属性器</strong>
          <button class="grid size-8 place-items-center rounded-lg border border-[#222228] bg-[#15151a] text-[#9ca3af] hover:text-white" type="button" title="关闭属性器" aria-label="关闭属性器" @click="closeInspector">
            <X :size="16" />
          </button>
        </header>

        <section class="grid content-start gap-6 overflow-y-auto px-4 py-[22px]">
        <div class="rounded-lg border border-[#222228] bg-[#121216] p-3.5">
          <h2 class="mb-2 text-[12px] font-bold leading-[15px] text-[#d1d5db]">AI 扩散设置</h2>
          <p class="text-[10px] leading-[1.55] text-[#6b7280]">基于 Diffusion 渲染架构，在保留输入特征的同时进行创造性发散。</p>
        </div>

        <div v-if="selectedCanvasNode" class="rounded-lg border border-[#222228] bg-[#121216] p-3.5">
          <div class="mb-3 flex items-center justify-between gap-3">
            <h2 class="truncate text-[12px] font-bold leading-[15px] text-[#d1d5db]">{{ nodeDefinitions[selectedCanvasNode.type].label }}</h2>
            <span class="rounded-full border border-[#222228] bg-[#0e0e11] px-2 py-1 text-[9px] font-bold text-[#6b7280]">{{ selectedCanvasNode.status }}</span>
          </div>
          <label v-if="getModelOptionsForNode(selectedCanvasNode).length" class="grid gap-1.5">
            <span class="text-[10px] font-extrabold text-[#6b7280]">当前模型</span>
            <select v-model="selectedCanvasNode.modelId" class="h-8 w-full rounded border border-[#222228] bg-[#15151a] px-[9px] text-[11px] text-[#d1d5db] outline-none focus:border-[#10b981] focus:ring-2 focus:ring-[#10b981]/20" aria-label="当前节点模型">
              <option v-for="model in getModelOptionsForNode(selectedCanvasNode)" :key="model.id" :value="model.id">
                {{ model.label }}
              </option>
            </select>
          </label>
          <div v-if="selectedNodeModel" class="mt-3 flex items-center justify-between gap-3 text-[10px] text-[#6b7280]">
            <span class="truncate">{{ selectedNodeModel.subtitle }}</span>
            <strong v-if="selectedCanvasNode" class="text-[#10b981]">{{ getNodeTokenEstimateLabel(selectedCanvasNode) }}</strong>
          </div>
        </div>

        <div class="rounded-lg border border-[#222228] bg-[#121216] p-3.5">
          <div class="mb-3 flex items-start justify-between gap-3">
            <div class="min-w-0">
              <h2 class="mb-1 text-[12px] font-bold leading-[15px] text-[#d1d5db]">摄影机控制</h2>
              <p class="truncate text-[10px] leading-[1.45] text-[#6b7280]">{{ cameraSettingSummary }}</p>
            </div>
            <span class="grid size-7 shrink-0 place-items-center rounded-md border border-[#115e59] bg-[#0f2f2e] text-[#14b8a6]">
              <Aperture :size="15" />
            </span>
          </div>
          <button class="inline-flex h-8 w-full items-center justify-center gap-2 rounded-lg border border-[#2f2f36] bg-[#15151a] px-3 text-[11px] font-bold text-[#d1d5db] hover:border-[#10b981] hover:text-[#10b981]" type="button" @click="openCameraPanel">
            <Camera :size="14" />
            打开摄影机控制
          </button>
        </div>

        <label class="grid gap-2" for="negative-prompt">
          <span class="text-[10px] font-extrabold text-[#6b7280]">负面排除词</span>
          <textarea id="negative-prompt" v-model="negativePrompt" class="h-[58px] w-full resize-none rounded border border-[#222228] bg-[#15151a] px-[9px] py-[7px] text-[11px] leading-[15px] text-[#d1d5db] outline-none focus:border-[#10b981] focus:ring-2 focus:ring-[#10b981]/20" rows="3"></textarea>
        </label>

        <label class="grid gap-3.5">
          <span class="flex items-center justify-between gap-3 text-[11px] text-[#9ca3af]">
            渲染迭代步数 (Steps)
            <strong class="font-bold text-[#10b981]">{{ renderSteps }}</strong>
          </span>
          <input
            v-model.number="renderSteps"
            class="range-control"
            type="range"
            min="1"
            max="55"
            :style="{ '--fill': `${renderProgress}%` }"
          />
        </label>

        <label class="grid gap-3.5">
          <span class="flex items-center justify-between gap-3 text-[11px] text-[#9ca3af]">
            无分类引导系数 (CFG)
            <strong class="font-bold text-[#10b981]">{{ cfgScale.toFixed(1) }}</strong>
          </span>
          <input
            v-model.number="cfgScale"
            class="range-control"
            type="range"
            min="1"
            max="14"
            step="0.5"
            :style="{ '--fill': `${cfgProgress}%` }"
          />
        </label>
        </section>

        <footer class="mt-auto grid gap-3.5 border-t border-[#222228] bg-[#0b0b0d] px-4 pb-3.5 pt-5">
        <div class="flex items-center justify-between gap-3">
          <span class="inline-flex items-center gap-1.5 text-[10px] text-[#6b7280]">
            <Cpu :size="13" />
            物理显卡：
          </span>
          <strong class="truncate text-right text-[10px] font-medium text-[#d1d5db]">Apple M3 Max / RTX 4090</strong>
        </div>
        <div class="flex items-center justify-between gap-3">
          <span class="inline-flex items-center gap-1.5 text-[10px] text-[#6b7280]">
            <Server :size="13" />
            监听端口：
          </span>
          <strong class="truncate text-right text-[10px] font-medium text-[#10b981]">localhost:5001</strong>
        </div>
        </footer>
      </aside>
    </Transition>

    <div
      v-if="isCameraPanelOpen"
      class="fixed inset-0 z-50 grid place-items-center overflow-y-auto bg-black/60 px-3 py-5 backdrop-blur-sm"
      role="dialog"
      aria-modal="true"
      aria-labelledby="camera-control-title"
      @click.self="closeCameraPanel"
    >
      <section class="flex max-h-[calc(100dvh_-_40px)] w-[min(1024px,calc(100vw_-_24px))] flex-col overflow-hidden rounded-xl border border-[#2f2f36] bg-[#18181b] shadow-[0_24px_80px_rgb(0_0_0/0.58)]">
        <header class="flex min-h-[58px] flex-wrap items-center justify-between gap-3 px-4 py-3 max-[760px]:px-3">
          <h2 id="camera-control-title" class="text-[16px] font-bold leading-5 text-white">摄影机控制</h2>
          <label class="grid min-w-[76px] gap-1" for="camera-preset">
            <span class="sr-only">摄影机预设</span>
            <select
              id="camera-preset"
              v-model="selectedCameraPreset"
              class="h-8 rounded-lg border border-[#3a3a42] bg-[#1c1c20] px-2.5 text-[12px] font-bold text-[#f3f4f6] outline-none focus:border-[#10b981] focus:ring-2 focus:ring-[#10b981]/20"
              @change="selectCameraPreset"
            >
              <option v-for="preset in cameraPresets" :key="preset.value" :value="preset.value">
                {{ preset.label }}
              </option>
            </select>
          </label>
        </header>

        <div class="grid gap-4 overflow-y-auto px-4 pb-4 max-[760px]:px-3">
          <div class="flex flex-wrap gap-2" role="tablist" aria-label="摄影机类型">
            <button
              v-for="tab in cameraTypeTabs"
              :key="tab.value"
              class="inline-flex h-10 items-center gap-2 rounded-lg border px-3 text-[13px] font-bold"
              :class="selectedCameraCategory === tab.value ? 'border-[#4b5563] bg-[#24242a] text-white' : 'border-[#32323a] bg-[#1b1b1f] text-[#d1d5db] hover:border-[#4b5563]'"
              type="button"
              role="tab"
              :aria-selected="selectedCameraCategory === tab.value"
              @click="selectCameraCategory(tab.value)"
            >
              <component :is="tab.icon" :size="15" />
              {{ tab.label }}
            </button>
          </div>

          <div class="grid grid-cols-4 gap-2.5 max-[920px]:grid-cols-2 max-[560px]:grid-cols-1">
            <label class="grid gap-2 rounded-xl border border-[#303037] bg-[#202024] p-2.5" for="camera-body">
              <span class="text-[11px] font-bold text-[#85858e]">机身</span>
              <select id="camera-body" v-model="selectedCameraBodyId" class="h-10 min-w-0 rounded-lg border border-[#35353d] bg-[#18181b] px-2.5 text-[13px] font-bold text-white outline-none focus:border-[#10b981] focus:ring-2 focus:ring-[#10b981]/20">
                <option v-for="camera in activeCameraBodies" :key="camera.id" :value="camera.id">
                  {{ camera.label }}
                </option>
              </select>
            </label>

            <label class="grid gap-2 rounded-xl border border-[#303037] bg-[#202024] p-2.5" for="camera-lens">
              <span class="text-[11px] font-bold text-[#85858e]">镜头</span>
              <select id="camera-lens" v-model="selectedLens" class="h-10 min-w-0 rounded-lg border border-[#35353d] bg-[#18181b] px-2.5 text-[13px] font-bold text-white outline-none focus:border-[#10b981] focus:ring-2 focus:ring-[#10b981]/20">
                <option v-for="lens in lensOptions" :key="lens.value" :value="lens.value">
                  {{ lens.label }}
                </option>
              </select>
            </label>

            <label class="grid gap-2 rounded-xl border border-[#303037] bg-[#202024] p-2.5" for="camera-focal-length">
              <span class="text-[11px] font-bold text-[#85858e]">焦段</span>
              <select id="camera-focal-length" v-model="selectedFocalLength" class="h-10 min-w-0 rounded-lg border border-[#35353d] bg-[#18181b] px-2.5 text-[13px] font-bold text-white outline-none focus:border-[#10b981] focus:ring-2 focus:ring-[#10b981]/20">
                <option v-for="focalLength in focalLengthOptions" :key="focalLength" :value="focalLength">
                  {{ focalLength }}
                </option>
              </select>
            </label>

            <label class="grid gap-2 rounded-xl border border-[#303037] bg-[#202024] p-2.5" for="camera-aperture">
              <span class="text-[11px] font-bold text-[#85858e]">光圈</span>
              <select id="camera-aperture" v-model="selectedAperture" class="h-10 min-w-0 rounded-lg border border-[#35353d] bg-[#18181b] px-2.5 text-[13px] font-bold text-white outline-none focus:border-[#10b981] focus:ring-2 focus:ring-[#10b981]/20">
                <option v-for="aperture in apertureOptions" :key="aperture" :value="aperture">
                  {{ aperture }}
                </option>
              </select>
            </label>
          </div>

          <section class="rounded-xl border border-[#2f2f36] bg-[#1f1f23] px-4 py-3">
            <dl class="grid grid-cols-[92px_190px_1fr] gap-x-8 gap-y-1 border-b border-[#303037] pb-3 text-[12px] max-[760px]:grid-cols-1 max-[760px]:gap-y-2">
              <div class="min-w-0">
                <dt class="mb-1 text-[10px] font-bold text-[#85858e]">年份</dt>
                <dd class="font-bold text-[#60a5fa]">{{ activeCameraProfile.year }}</dd>
              </div>
              <div class="min-w-0">
                <dt class="mb-1 text-[10px] font-bold text-[#85858e]">画幅</dt>
                <dd class="font-bold text-[#60a5fa]">{{ activeCameraProfile.format }}</dd>
              </div>
              <div class="min-w-0">
                <dt class="mb-1 text-[10px] font-bold text-[#85858e]">传感器/胶片</dt>
                <dd class="font-bold text-[#cbd5e1]">{{ activeCameraProfile.sensor }}</dd>
              </div>
            </dl>

            <div class="border-b border-[#303037] py-3">
              <h3 class="mb-2 text-[13px] font-bold text-white">{{ activeCameraProfile.label }}</h3>
              <p class="text-[12px] leading-[1.7] text-[#cbd5e1]">{{ activeCameraProfile.description }}</p>
            </div>

            <p v-if="activeCameraProfile.works.length" class="flex flex-wrap items-center gap-1.5 pt-3 text-[12px] text-[#9ca3af]">
              <Clapperboard :size="13" class="text-[#cbd5e1]" />
              <span>经典作品：</span>
              <strong class="font-bold text-[#facc15]">{{ activeCameraProfile.works.join("、") }}</strong>
            </p>
          </section>

          <section class="grid gap-2">
            <div class="flex flex-wrap items-center gap-2 text-[12px]">
              <h3 class="font-bold text-white">镜头特效</h3>
              <span class="text-[#85858e]">{{ activeLensProfile.description }}</span>
            </div>
            <div class="flex flex-wrap gap-2" role="group" aria-label="镜头特效">
              <button
                v-for="effect in lensEffects"
                :key="effect.value"
                class="relative inline-flex h-8 items-center rounded-full border px-3 text-[12px] font-bold"
                :class="selectedLensEffect === effect.value ? 'border-[#4b5563] bg-[#2a2a30] text-white' : 'border-[#36363e] bg-[#202024] text-[#d1d5db] hover:border-[#4b5563]'"
                type="button"
                :title="effect.description"
                @click="selectedLensEffect = effect.value"
              >
                <span v-if="selectedLensEffect === effect.value" class="absolute -left-0.5 -top-0.5 size-2.5 rounded-full border border-white bg-[#f472b6]"></span>
                {{ effect.label }}
              </button>
            </div>
          </section>
        </div>

        <footer class="flex justify-end gap-2 border-t border-[#2f2f36] px-4 py-4 max-[560px]:grid max-[560px]:grid-cols-2">
          <button class="inline-flex h-9 min-w-[84px] items-center justify-center rounded-lg border border-[#4b4b55] bg-[#1f1f23] px-4 text-[12px] font-bold text-white hover:border-[#6b7280]" type="button" @click="closeCameraPanel">
            关闭
          </button>
          <button class="inline-flex h-9 min-w-[84px] items-center justify-center rounded-lg border border-[#5a5a64] bg-[#4a4a51] px-4 text-[12px] font-bold text-white hover:bg-[#5a5a62]" type="button" @click="applyCameraControl">
            应用
          </button>
        </footer>
      </section>
    </div>

    <div
      v-if="isApiKeyPanelOpen"
      class="fixed inset-0 z-50 grid place-items-center overflow-y-auto bg-black/60 px-4 py-6 backdrop-blur-sm"
      role="dialog"
      aria-modal="true"
      aria-labelledby="api-key-title"
      @click.self="closeApiKeyPanel"
    >
      <form class="flex max-h-[calc(100dvh_-_48px)] w-[min(520px,calc(100vw_-_32px))] flex-col overflow-hidden rounded-xl border border-[#222228] bg-[#101014] shadow-[0_24px_80px_rgb(0_0_0/0.52)]" @submit.prevent="saveApiKeySettings">
        <header class="grid grid-cols-[1fr_auto] items-center gap-4 border-b border-[#222228] px-5 py-4">
          <div class="min-w-0">
            <p class="mb-1 text-[10px] font-extrabold leading-3 text-[#4b5563]">SETTINGS</p>
            <h2 id="api-key-title" class="truncate text-[16px] font-bold leading-5 text-white">API Key 设置</h2>
          </div>
          <button class="grid size-8 place-items-center rounded-lg border border-[#222228] bg-[#15151a] text-[#9ca3af] hover:text-white" type="button" title="关闭" @click="closeApiKeyPanel">
            <X :size="16" />
          </button>
        </header>

        <section class="grid gap-5 overflow-y-auto px-5 py-5">
          <div class="grid gap-2">
            <span class="text-[10px] font-extrabold leading-[14px] text-[#6b7280]">服务商</span>
            <div class="grid h-9 grid-cols-3 gap-0.5 rounded-[5px] border border-[#222228] bg-[#15151a] p-0.5" role="group" aria-label="API provider">
              <button
                v-for="provider in apiProviderOptions"
                :key="provider.value"
                type="button"
                class="rounded text-[10px] font-extrabold"
                :class="apiKeyDraft.provider === provider.value ? 'border border-[#115e59] bg-[#0f2f2e] text-[#14b8a6]' : 'border border-transparent text-[#6b7280]'"
                @click="selectApiProvider(provider.value)"
              >
                {{ provider.label }}
              </button>
            </div>
          </div>

          <label class="grid gap-2" for="api-base-url">
            <span class="text-[10px] font-extrabold leading-[14px] text-[#6b7280]">Base URL</span>
            <input
              id="api-base-url"
              v-model="apiKeyDraft.baseUrl"
              class="h-10 w-full rounded border border-[#222228] bg-[#15151a] px-3 text-[12px] text-[#d1d5db] outline-none focus:border-[#10b981] focus:ring-2 focus:ring-[#10b981]/20"
              inputmode="url"
              spellcheck="false"
            />
          </label>

          <label class="grid gap-2" for="api-key-input">
            <span class="text-[10px] font-extrabold leading-[14px] text-[#6b7280]">API Key</span>
            <span class="relative block">
              <input
                id="api-key-input"
                v-model="apiKeyDraft.apiKey"
                class="h-10 w-full rounded border border-[#222228] bg-[#15151a] px-3 pr-11 text-[12px] text-[#d1d5db] outline-none focus:border-[#10b981] focus:ring-2 focus:ring-[#10b981]/20"
                :type="showApiKey ? 'text' : 'password'"
                autocomplete="off"
                spellcheck="false"
              />
              <button
                class="absolute right-1.5 top-1/2 grid size-7 -translate-y-1/2 place-items-center rounded-md text-[#6b7280] hover:bg-[#1d1d22] hover:text-[#d1d5db]"
                type="button"
                :title="showApiKey ? '隐藏 API Key' : '显示 API Key'"
                @click="showApiKey = !showApiKey"
              >
                <EyeOff v-if="showApiKey" :size="15" />
                <Eye v-else :size="15" />
              </button>
            </span>
          </label>

          <div class="flex min-h-9 items-center gap-2 rounded border border-[#222228] bg-[#0d0d10] px-3 text-[11px] text-[#9ca3af]">
            <KeyRound :size="14" class="shrink-0 text-[#10b981]" />
            <span class="min-w-0 truncate">{{ apiKeyMessage }}</span>
          </div>
        </section>

        <footer class="flex flex-wrap items-center justify-between gap-3 border-t border-[#222228] px-5 py-4">
          <button
            class="inline-flex h-9 min-w-[92px] items-center justify-center gap-2 rounded-lg border border-[#2f2f36] bg-[#15151a] px-3 text-[12px] font-bold text-[#9ca3af] hover:text-white disabled:cursor-not-allowed disabled:opacity-45"
            type="button"
            :disabled="isClearingApiKey || !hasSavedApiKey"
            @click="clearApiKeySettings"
          >
            <Trash2 :size="14" />
            {{ isClearingApiKey ? "清除中..." : "清除" }}
          </button>
          <button class="inline-flex h-9 min-w-[116px] items-center justify-center gap-2 rounded-lg border-0 bg-[#10b981] px-4 text-[12px] font-black text-[#070708] hover:brightness-110 disabled:cursor-wait disabled:opacity-75" type="submit" :disabled="isSavingApiKey">
            <Save :size="14" />
            {{ isSavingApiKey ? "保存中..." : "保存配置" }}
          </button>
        </footer>
      </form>
    </div>
  </main>
</template>
