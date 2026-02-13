<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { Crepe } from '@milkdown/crepe';
import { Pin, Minus, X } from 'lucide-vue-next';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { invoke } from '@tauri-apps/api/core';

// Import Crepe styles
import '@milkdown/crepe/theme/common/style.css';
import '@milkdown/crepe/theme/frame.css';

const editorRef = ref<HTMLDivElement | null>(null);
const isAlwaysOnTop = ref(false); 
const appWindow = getCurrentWindow();
const noteId = ref('');
const currentContent = ref('');

// Simple debounce
let saveTimeout: number | null = null;
const debounceSave = (content: string) => {
  if (saveTimeout) clearTimeout(saveTimeout);
  saveTimeout = setTimeout(async () => {
    try {
      await invoke('save_note', { id: noteId.value, content });
      saveTimeout = null;
    } catch (e) {
      console.error('Failed to save note:', e);
    }
  }, 1000) as unknown as number;
};

onMounted(async () => {
  const label = appWindow.label;
  noteId.value = label.startsWith('note-') ? label.replace('note-', '') : label;

  let initialContent = '';
  try {
    const saved = await invoke<string>('load_note', { id: noteId.value });
    if (saved) {
      initialContent = saved;
      currentContent.value = saved;
    }
  } catch (e) {
    console.error('Failed to load note:', e);
  }

  if (editorRef.value) {
    const crepe = new Crepe({
      root: editorRef.value,
      defaultValue: initialContent,
      features: {
        [Crepe.Feature.Placeholder]: false,
      },
    });

    crepe.on((listener) => {
      listener.markdownUpdated((_ctx: any, markdown: string) => {
        currentContent.value = markdown;
        debounceSave(markdown);
      });
    });

    await crepe.create();
  }
});

const toggleAlwaysOnTop = async () => {
  isAlwaysOnTop.value = !isAlwaysOnTop.value;
  await appWindow.setAlwaysOnTop(isAlwaysOnTop.value);
};

const minimizeWindow = async () => {
  await appWindow.minimize();
};

const closeWindow = async () => {
  // If there's a pending save, execute it now
  if (saveTimeout) {
    clearTimeout(saveTimeout);
    saveTimeout = null;
    try {
      await invoke('save_note', { id: noteId.value, content: currentContent.value });
    } catch (e) {
      console.error('Failed to save note on close:', e);
    }
  }
  // Properly close the window so it's destroyed and removed from session
  await appWindow.close();
};
</script>

<template>
  <div class="sticky-note">
    <div class="drag-handle" data-tauri-drag-region>
      <div class="spacer" data-tauri-drag-region></div>
      <div class="controls">
        <button 
          class="control-btn pin-btn" 
          :class="{ active: isAlwaysOnTop }" 
          title="Toggle Always on Top"
          @click="toggleAlwaysOnTop"
        >
          <Pin :size="14" />
        </button>
        <button 
          class="control-btn" 
          title="Minimize"
          @click="minimizeWindow"
        >
          <Minus :size="14" />
        </button>
        <button 
          class="control-btn close-btn" 
          title="Close"
          @click="closeWindow"
        >
          <X :size="14" />
        </button>
      </div>
    </div>
    <div class="editor-container">
      <div ref="editorRef" class="crepe-editor"></div>
    </div>
  </div>
</template>

<style>
html, body {
  margin: 0;
  padding: 0;
  width: 100vw;
  height: 100vh;
  /* Updated Background Color */
  background-color: #FFF6CB !important; 
  overflow: hidden;
  font-family: sans-serif;
}

#app {
  width: 100%;
  height: 100%;
}

.sticky-note {
  display: flex;
  flex-direction: column;
  width: 100%;
  height: 100%;
  box-sizing: border-box;
  /* Updated border to match theme */
  border: 1px solid #E5D058;
}

.drag-handle {
  height: 32px;
  /* Updated Titlebar Color */
  background-color: #F2DD65 !important;
  display: flex;
  align-items: center;
  justify-content: flex-end;
  padding: 0 4px;
  cursor: move;
  user-select: none;
  flex-shrink: 0;
}

.spacer {
  flex: 1;
  height: 100%;
}

.controls {
  display: flex;
  gap: 2px;
  align-items: center;
}

.control-btn {
  background: transparent;
  border: none;
  outline: none;
  cursor: pointer;
  width: 28px;
  height: 28px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #854d0e;
  opacity: 0.6;
  border-radius: 4px;
  transition: all 0.2s ease;
}

.control-btn:hover {
  opacity: 1;
  background-color: rgba(133, 77, 14, 0.1);
}

.control-btn.active {
  opacity: 1;
  color: #ca8a04;
  background-color: rgba(202, 138, 4, 0.1);
}

.close-btn:hover {
  color: #ef4444;
  background-color: rgba(239, 68, 68, 0.1);
}

.editor-container {
  flex: 1;
  overflow-y: auto;
  padding: 0;
  /* Hide scrollbar for Firefox */
  scrollbar-width: none;
}

/* Hide scrollbar for Chrome, Safari and Opera */
.editor-container::-webkit-scrollbar {
  display: none;
}

.crepe-editor {
  height: 100%;
  width: 100%;
}

.milkdown {
  height: 100% !important;
  max-height: 100% !important;
  background-color: transparent !important;
}

.milkdown .editor {
  height: 100% !important;
  padding: 15px !important;
  outline: none !important;
  background-color: transparent !important;
  box-shadow: none !important;
  box-sizing: border-box;
  color: #1e293b !important;
}

.milkdown-crepe-frame {
  border: none !important;
  background-color: transparent !important;
  box-shadow: none !important;
  padding: 0 !important;
  height: 100% !important;
}

/* Hide any remaining placeholders */
.milkdown .placeholder {
  display: none !important;
}
</style>
