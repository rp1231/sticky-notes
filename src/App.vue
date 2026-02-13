<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { Crepe } from '@milkdown/crepe';
import { Pin, Minus, X, LayoutDashboard, Plus, Trash2, ExternalLink, RefreshCw } from 'lucide-vue-next';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

// Import Crepe styles
import '@milkdown/crepe/theme/common/style.css';
import '@milkdown/crepe/theme/frame.css';

interface NoteInfo {
  id: string;
  preview: string;
}

const editorRef = ref<HTMLDivElement | null>(null);
const isAlwaysOnTop = ref(false); 
const appWindow = getCurrentWindow();
const isDashboard = ref(appWindow.label === 'main');
const isCreatingNote = ref(false);
const noteId = ref('');
const currentContent = ref('');
const allNotes = ref<NoteInfo[]>([]);

// Simple debounce
let saveTimeout: number | null = null;
const debounceSave = (content: string) => {
  if (saveTimeout) clearTimeout(saveTimeout);
  saveTimeout = setTimeout(async () => {
    try {
      await invoke('save_note', { id: noteId.value, content });
      saveTimeout = null;
      // Trigger a refresh on dashboard
      await invoke('trigger_refresh_notes');
    } catch (e) {
      console.error('Failed to save note:', e);
    }
  }, 1000) as unknown as number;
};

const fetchNotes = async () => {
  try {
    allNotes.value = await invoke<NoteInfo[]>('get_all_notes');
  } catch (e) {
    console.error('Failed to fetch notes:', e);
  }
};

onMounted(async () => {
  const label = appWindow.label;
  console.log('Window label:', label);
  
  if (isDashboard.value) {
    console.log('Dashboard detected');
    await fetchNotes();
    await listen('refresh-notes', () => {
      console.log('Received refresh-notes event');
      fetchNotes();
    });
    return;
  }

  console.log('Note window detected, ID:', label);
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
  if (isDashboard.value) {
    await appWindow.hide();
    return;
  }

  // If there's a pending save, execute it now
  if (saveTimeout) {
    clearTimeout(saveTimeout);
    saveTimeout = null;
    try {
      await invoke('save_note', { id: noteId.value, content: currentContent.value });
      await invoke('trigger_refresh_notes');
    } catch (e) {
      console.error('Failed to save note on close:', e);
    }
  }
  // Properly close the window so it's destroyed and removed from session
  await appWindow.close();
};

const createNewNote = async () => {
  if (isCreatingNote.value) return;

  console.log('Frontend: createNewNote clicked');
  isCreatingNote.value = true;
  try {
    await invoke('create_new_note_cmd');
    console.log('Frontend: create_new_note_cmd finished');
  } catch (e) {
    console.error('Frontend: Failed to create new note:', e);
  } finally {
    isCreatingNote.value = false;
  }
};

const deleteNote = async (id: string) => {
  if (confirm('Permanently delete this note?')) {
    try {
      await invoke('delete_note', { id });
    } catch (e) {
      console.error('Failed to delete note:', e);
    }
  }
};

const openNoteWindow = async (id: string) => {
  console.log('Frontend: openNoteWindow', id);
  try {
    await invoke('open_note_window_cmd', { id });
  } catch (e) {
    console.error('Failed to open note window:', e);
  }
};
</script>

<template>
  <div class="sticky-note" :class="{ 'is-dashboard': isDashboard }">
    <div class="drag-handle" data-tauri-drag-region>
      <div class="title-area" data-tauri-drag-region>
        <LayoutDashboard v-if="isDashboard" :size="16" class="title-icon" />
        <span class="title-text" data-tauri-drag-region>{{ isDashboard ? 'Notes Dashboard' : 'Sticky Note' }}</span>
      </div>
      <div class="controls">
        <button 
          v-if="isDashboard"
          class="control-btn" 
          title="Refresh List"
          @click.stop="fetchNotes"
        >
          <RefreshCw :size="14" />
        </button>
        <button 
          v-if="!isDashboard"
          class="control-btn pin-btn" 
          :class="{ active: isAlwaysOnTop }" 
          title="Toggle Always on Top"
          @click.stop="toggleAlwaysOnTop"
        >
          <Pin :size="14" />
        </button>
        <button 
          class="control-btn" 
          title="Minimize"
          @click.stop="minimizeWindow"
        >
          <Minus :size="14" />
        </button>
        <button 
          class="control-btn close-btn" 
          title="Close"
          @click.stop="closeWindow"
        >
          <X :size="14" />
        </button>
      </div>
    </div>


    <!-- Dashboard View -->
    <div v-if="isDashboard" class="dashboard-content">
      <div class="dashboard-header">
        <button 
          class="new-note-card" 
          :class="{ 'is-loading': isCreatingNote }"
          :disabled="isCreatingNote"
          @click="createNewNote"
        >
          <Plus v-if="!isCreatingNote" :size="24" />
          <RefreshCw v-else :size="24" class="spin" />
          <span>{{ isCreatingNote ? 'Creating...' : 'New Note' }}</span>
        </button>
      </div>
      <div class="notes-grid">
        <div v-for="note in allNotes" :key="note.id" class="note-card">
          <div class="note-preview">
            {{ note.preview || 'Empty Note' }}
          </div>
          <div class="note-footer">
            <button class="note-action-btn open" title="Open Note" @click="openNoteWindow(note.id)">
              <ExternalLink :size="14" />
            </button>
            <button class="note-action-btn delete" title="Delete Permanently" @click="deleteNote(note.id)">
              <Trash2 :size="14" />
            </button>
          </div>
        </div>
      </div>
      <div v-if="allNotes.length === 0" class="empty-state">
        No saved notes yet. Create one above!
      </div>
    </div>

    <!-- Editor View -->
    <div v-else class="editor-container">
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
  background-color: #FFF6CB !important; 
  overflow: hidden;
  font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
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
  border: 1px solid #E5D058;
}

.sticky-note.is-dashboard {
  background-color: #fefce8;
}

.drag-handle {
  height: 32px;
  background-color: #F2DD65 !important;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 8px;
  cursor: move;
  user-select: none;
  flex-shrink: 0;
}

.title-area {
  display: flex;
  align-items: center;
  gap: 8px;
  color: #854d0e;
  font-size: 12px;
  font-weight: 600;
}

.title-icon {
  opacity: 0.7;
}

.controls {
  display: flex;
  gap: 2px;
  align-items: center;
  position: relative;
  z-index: 50;
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
  pointer-events: auto;
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

/* Dashboard Styles */
.dashboard-content {
  flex: 1;
  overflow-y: auto;
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.notes-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
  gap: 12px;
}

.new-note-card {
  width: 100%;
  height: 60px;
  background-color: #fef9c3;
  border: 2px dashed #eab308;
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 12px;
  color: #a16207;
  cursor: pointer;
  transition: all 0.2s ease;
  font-weight: 600;
}

.new-note-card:hover {
  background-color: #fef08a;
  border-color: #ca8a04;
  transform: translateY(-1px);
}

.new-note-card:disabled {
  opacity: 0.7;
  cursor: not-allowed;
  transform: none;
}

.spin {
  animation: spin 1s linear infinite;
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

.empty-state {
  text-align: center;
  padding: 40px;
  color: #854d0e;
  opacity: 0.5;
  font-style: italic;
  font-size: 14px;
}

.note-card {
  background-color: #fff;
  border: 1px solid #fef08a;
  border-radius: 8px;
  height: 160px;
  display: flex;
  flex-direction: column;
  box-shadow: 0 2px 4px rgba(0,0,0,0.05);
  transition: all 0.2s ease;
}

.note-card:hover {
  box-shadow: 0 4px 12px rgba(0,0,0,0.1);
  transform: translateY(-2px);
}

.note-preview {
  flex: 1;
  padding: 12px;
  font-size: 11px;
  color: #475569;
  overflow: hidden;
  display: -webkit-box;
  -webkit-line-clamp: 7;
  -webkit-box-orient: vertical;
  line-height: 1.4;
  white-space: pre-wrap;
}

.note-footer {
  height: 36px;
  border-top: 1px solid #f1f5f9;
  display: flex;
  align-items: center;
  justify-content: flex-end;
  padding: 0 8px;
  gap: 4px;
  background-color: #fafafa;
  border-radius: 0 0 8px 8px;
}

.note-action-btn {
  width: 28px;
  height: 28px;
  border-radius: 4px;
  border: none;
  background: transparent;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  color: #64748b;
  transition: all 0.2s ease;
}

.note-action-btn.open:hover {
  background-color: #eff6ff;
  color: #3b82f6;
}

.note-action-btn.delete:hover {
  background-color: #fef2f2;
  color: #ef4444;
}

/* Editor Styles */
.editor-container {
  flex: 1;
  overflow-y: auto;
  padding: 0;
  scrollbar-width: none;
}

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

.milkdown .placeholder {
  display: none !important;
}
</style>
