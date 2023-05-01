<script setup>

import { onMounted } from 'vue'
import 'xterm/css/xterm.css';
import { Terminal } from 'xterm';
import { FitAddon } from 'xterm-addon-fit';
import { AttachAddon } from 'xterm-addon-attach';
import { useRouter, useRoute } from 'vue-router'
    
const router = useRouter()
const route = useRoute()

const term = new Terminal({cursorBlink: true, convertEol: true, scrollback: 6000});
const fitAddon = new FitAddon();

var socket;
var command;

// lifecycle hooks
onMounted(() => {
  socket = new WebSocket("ws://localhost:3000/cfssession/" + route.params.cfssession + "/logs");

  const attachAddon = new AttachAddon(socket);

  term.loadAddon(attachAddon);
  term.loadAddon(fitAddon);

  term.open(document.getElementById('terminal'));

  fitAddon.fit();
})
</script>

<template>
  <div class="cfssessionlogs">
    <h1>CFS Session logs - {{ $route.params.cfssession }}</h1>
    <div id="terminal"></div>
  </div>
</template>

<style>
</style>
