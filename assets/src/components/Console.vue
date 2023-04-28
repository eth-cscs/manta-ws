<script setup>

import { onMounted } from 'vue'
import 'xterm/css/xterm.css';
import { Terminal } from 'xterm';
import { FitAddon } from 'xterm-addon-fit';
import { AttachAddon } from 'xterm-addon-attach';
import { useRouter, useRoute } from 'vue-router'
    
const router = useRouter()
const route = useRoute()

const term = new Terminal({cursorBlink: true, convertEol: true});
const fitAddon = new FitAddon();

var socket;
var command;

// lifecycle hooks
onMounted(() => {
  socket = new WebSocket("ws://localhost:3000/console/" + route.params.xname);

  const attachAddon = new AttachAddon(socket);

  term.loadAddon(attachAddon);
  term.loadAddon(fitAddon);

  term.open(document.getElementById('terminal'));
})
</script>

<template>
  <div class="console">
    <h1>Node {{ $route.params.xname }} - Console</h1>
    <div id="terminal"></div>
  </div>
</template>

<style>
</style>
