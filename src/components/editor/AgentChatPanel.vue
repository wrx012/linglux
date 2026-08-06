<script setup lang="ts">
import { nextTick, ref, watch } from "vue";
import {
  AlertTriangle,
  Ban,
  Bot,
  Check,
  Send,
  Sparkles,
  Square,
  Trash2,
  X,
} from "@lucide/vue";
import { previewAgentEditPlan } from "../../lib/editorAgent";
import type {
  AgentConversation,
  AgentEditPlan,
  AgentPlanState,
} from "../../types/agent";
import type { EditorProject } from "../../types/editor";

const props = defineProps<{
  project: EditorProject;
  editorVersion: number;
  conversation: AgentConversation;
  isRunning: boolean;
  status: string;
  error: string;
  isDesktop: boolean;
}>();

const emit = defineEmits<{
  close: [];
  send: [prompt: string];
  cancel: [];
  applyPlan: [plan: AgentEditPlan];
  rejectPlan: [plan: AgentEditPlan];
  clear: [];
  openSettings: [];
}>();

const prompt = ref("");
const messageScroller = ref<HTMLElement>();

watch(
  () => props.conversation.messages.length,
  () => {
    void nextTick(() => {
      const scroller = messageScroller.value;
      if (scroller) {
        scroller.scrollTop = scroller.scrollHeight;
      }
    });
  },
  { immediate: true },
);

function submitPrompt() {
  const value = prompt.value.trim();

  if (!value || props.isRunning || !props.isDesktop) {
    return;
  }

  emit("send", value);
  prompt.value = "";
}

function handlePromptKeydown(event: KeyboardEvent) {
  if (event.key === "Enter" && !event.shiftKey) {
    event.preventDefault();
    submitPrompt();
  }
}

function planState(plan: AgentEditPlan, storedState?: AgentPlanState) {
  if (storedState === "applied" || storedState === "rejected") {
    return storedState;
  }

  return plan.baseEditorVersion === props.editorVersion ? storedState ?? "pending" : "stale";
}

function planDescriptions(plan: AgentEditPlan) {
  try {
    return previewAgentEditPlan(props.project, plan, props.editorVersion).descriptions;
  } catch (error) {
    return [error instanceof Error ? error.message : String(error)];
  }
}

function confirmClear() {
  if (window.confirm("清空当前工程的 AI 对话记录？已应用的时间线改动不会被删除。")) {
    emit("clear");
  }
}
</script>

<template>
  <aside class="flex h-full min-h-0 flex-col border-l border-[#232936] bg-[#0e131c] text-[#dbe4f1]" aria-label="AI 剪辑助手">
    <header class="flex h-14 shrink-0 items-center gap-3 border-b border-[#232936] px-3">
      <span class="grid size-8 shrink-0 place-items-center rounded-lg bg-[#102f2d] text-[#5eead4] ring-1 ring-[#1f766d]/50">
        <Sparkles :size="15" />
      </span>
      <span class="min-w-0 flex-1">
        <strong class="block truncate text-[12px] font-black">Linglux Agent</strong>
        <span class="block truncate text-[9px] font-semibold text-[#748096]">只生成受控计划，不操作鼠标</span>
      </span>
      <UButton color="neutral" variant="ghost" square size="xs" type="button" aria-label="清空 AI 对话" :disabled="conversation.messages.length === 0 || isRunning" @click="confirmClear">
        <Trash2 :size="13" />
      </UButton>
      <UButton color="neutral" variant="ghost" square size="xs" type="button" aria-label="关闭 AI 剪辑助手" @click="emit('close')">
        <X :size="14" />
      </UButton>
    </header>

    <div ref="messageScroller" class="min-h-0 flex-1 space-y-3 overflow-y-auto px-3 py-4">
      <div v-if="conversation.messages.length === 0" class="rounded-xl border border-[#263044] bg-[#111927] p-4">
        <div class="mb-3 flex items-center gap-2 text-[#7dd3fc]">
          <Bot :size="16" />
          <strong class="text-[11px] font-black">用自然语言编辑时间线</strong>
        </div>
        <p class="text-[10px] font-semibold leading-5 text-[#8793a8]">
          例如：把 x.mp4 加到主视频轨，1:03 前面的不要，3:02 后面的不要。
        </p>
        <p class="mt-2 text-[9px] font-semibold leading-4 text-[#5f6b7e]">
          Agent 会先展示计划，只有你确认后才修改工程。
        </p>
      </div>

      <article
        v-for="message in conversation.messages"
        :key="message.id"
        class="rounded-xl border p-3"
        :class="message.role === 'user' ? 'ml-6 border-[#244167] bg-[#12233b]' : 'mr-2 border-[#263044] bg-[#111927]'"
      >
        <div class="mb-1.5 flex items-center gap-2">
          <span class="text-[9px] font-black uppercase tracking-[0.14em]" :class="message.role === 'user' ? 'text-[#60a5fa]' : 'text-[#5eead4]'">
            {{ message.role === "user" ? "你" : "Agent" }}
          </span>
        </div>
        <p class="whitespace-pre-wrap text-[10px] font-semibold leading-5 text-[#cbd5e1]">{{ message.content }}</p>

        <section v-if="message.plan" class="mt-3 overflow-hidden rounded-lg border border-[#315249] bg-[#0b1d1b]">
          <div class="flex items-center justify-between gap-2 border-b border-[#29453f] px-3 py-2">
            <strong class="text-[9px] font-black uppercase tracking-[0.14em] text-[#5eead4]">剪辑计划</strong>
            <UBadge
              :color="planState(message.plan, message.planState) === 'applied' ? 'success' : planState(message.plan, message.planState) === 'pending' ? 'warning' : 'neutral'"
              variant="subtle"
              size="sm"
              class="text-[8px]"
            >
              {{
                planState(message.plan, message.planState) === "applied"
                  ? "已应用"
                  : planState(message.plan, message.planState) === "rejected"
                    ? "已拒绝"
                    : planState(message.plan, message.planState) === "stale"
                      ? "已过期"
                      : "待确认"
              }}
            </UBadge>
          </div>
          <ol class="space-y-1.5 px-3 py-2.5">
            <li v-for="(description, index) in planDescriptions(message.plan)" :key="`${message.plan.id}-${index}`" class="flex gap-2 text-[9px] font-semibold leading-4 text-[#aab7c8]">
              <span class="mt-0.5 grid size-4 shrink-0 place-items-center rounded-full bg-[#173b36] text-[8px] font-black text-[#6ee7b7]">{{ index + 1 }}</span>
              <span>{{ description }}</span>
            </li>
          </ol>
          <div v-if="message.plan.warnings.length > 0" class="mx-3 mb-2 flex gap-2 rounded-md border border-[#7c5b1f]/60 bg-[#2c210d] px-2 py-1.5 text-[8px] font-semibold leading-4 text-[#facc67]">
            <AlertTriangle :size="12" class="mt-0.5 shrink-0" />
            <span>{{ message.plan.warnings.join("；") }}</span>
          </div>
          <div v-if="planState(message.plan, message.planState) === 'pending'" class="grid grid-cols-2 gap-2 border-t border-[#29453f] p-2">
            <UButton color="neutral" variant="outline" size="xs" type="button" class="justify-center text-[9px]" :disabled="isRunning" @click="emit('rejectPlan', message.plan)">
              <Ban :size="12" />
              拒绝
            </UButton>
            <UButton color="primary" variant="solid" size="xs" type="button" class="justify-center text-[9px] font-black" :disabled="isRunning" @click="emit('applyPlan', message.plan)">
              <Check :size="12" />
              应用计划
            </UButton>
          </div>
          <p v-else-if="planState(message.plan, message.planState) === 'stale'" class="border-t border-[#5b3a2a] bg-[#271711] px-3 py-2 text-[8px] font-bold text-[#fb923c]">
            时间线已变化，请重新发送指令生成新计划。
          </p>
        </section>
      </article>

      <div v-if="isRunning" class="flex items-center gap-2 rounded-lg border border-[#264558] bg-[#10202b] px-3 py-2 text-[9px] font-bold text-[#7dd3fc]" role="status">
        <span class="size-2 animate-pulse rounded-full bg-[#38bdf8]"></span>
        {{ status || "正在理解剪辑指令…" }}
      </div>
      <div v-if="error" class="rounded-lg border border-[#713642] bg-[#2b141b] px-3 py-2 text-[9px] font-semibold leading-4 text-[#fda4af]" role="alert">
        {{ error }}
      </div>
    </div>

    <footer class="shrink-0 border-t border-[#232936] p-3">
      <div v-if="!isDesktop" class="mb-2 rounded-md border border-[#60471d] bg-[#291f0e] px-2.5 py-2 text-[9px] font-semibold leading-4 text-[#fbbf24]">
        真实模型调用仅在 Tauri 桌面应用中可用。
      </div>
      <div class="rounded-xl border border-[#2a3445] bg-[#111823] p-2 focus-within:border-[#2f7e74]">
        <textarea
          v-model="prompt"
          class="min-h-[72px] w-full resize-none bg-transparent px-1 py-1 text-[10px] font-semibold leading-5 text-[#dbe4f1] outline-none placeholder:text-[#566174]"
          placeholder="描述你想怎样剪辑…"
          :disabled="isRunning || !isDesktop"
          aria-label="AI 剪辑指令"
          @keydown="handlePromptKeydown"
        ></textarea>
        <div class="flex items-center justify-between gap-2 pt-1">
          <UButton color="neutral" variant="ghost" size="xs" type="button" class="text-[9px]" @click="emit('openSettings')">
            模型设置
          </UButton>
          <UButton v-if="isRunning" color="error" variant="soft" square size="xs" type="button" aria-label="停止 AI 请求" @click="emit('cancel')">
            <Square :size="12" />
          </UButton>
          <UButton v-else color="primary" variant="solid" square size="xs" type="button" aria-label="发送 AI 剪辑指令" :disabled="!prompt.trim() || !isDesktop" @click="submitPrompt">
            <Send :size="13" />
          </UButton>
        </div>
      </div>
      <p class="mt-2 text-center text-[8px] font-semibold text-[#4f5b6d]">Enter 发送 · Shift+Enter 换行 · 应用前始终预览</p>
    </footer>
  </aside>
</template>
