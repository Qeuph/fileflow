<script lang="ts">
  import { SvelteFlow, Controls, Background, MiniMap } from '@xyflow/svelte';
  import '@xyflow/svelte/dist/style.css';
  import { File, Image, CloudUpload } from 'lucide-svelte';

  let nodes = $state([
    {
      id: '1',
      type: 'input',
      data: { label: 'File Watcher' },
      position: { x: 250, y: 0 }
    },
    {
      id: '2',
      data: { label: 'Image Processor' },
      position: { x: 250, y: 150 }
    },
    {
      id: '3',
      type: 'output',
      data: { label: 'S3 Upload' },
      position: { x: 250, y: 300 }
    }
  ]);

  let edges = $state([
    { id: 'e1-2', source: '1', target: '2', animated: true },
    { id: 'e2-3', source: '2', target: '3' }
  ]);
</script>

<div class="flow-container">
  <SvelteFlow bind:nodes bind:edges fitView>
    <Controls />
    <Background />
    <MiniMap />
  </SvelteFlow>
  
  <div class="toolbar">
    <div class="icon-item"><File size={20} /> Watcher</div>
    <div class="icon-item"><Image size={20} /> Processor</div>
    <div class="icon-item"><CloudUpload size={20} /> S3</div>
  </div>
</div>

<style>
  .flow-container {
    width: 100vw;
    height: 100vh;
    position: relative;
    background: #1a1a1a;
  }

  :global(.svelte-flow) {
    background: #1a1a1a;
  }

  :global(.svelte-flow__node) {
    background: #2a2a2a;
    color: white;
    border: 1px solid #444;
    border-radius: 8px;
  }

  .toolbar {
    position: absolute;
    top: 10px;
    right: 10px;
    background: rgba(0, 0, 0, 0.7);
    padding: 10px;
    border-radius: 8px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    z-index: 5;
    color: white;
  }

  .icon-item {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 14px;
  }
</style>
