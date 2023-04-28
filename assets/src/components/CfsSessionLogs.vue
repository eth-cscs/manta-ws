<script setup>

import { onMounted } from 'vue'
import 'xterm/css/xterm.css';
import { Terminal } from 'xterm';
import { FitAddon } from 'xterm-addon-fit';
import { useRouter, useRoute } from 'vue-router'
    
const router = useRouter()
const route = useRoute()

const term = new Terminal({cursorBlink: true, convertEol: true});
const fitAddon = new FitAddon();
term.loadAddon(fitAddon);

var socket;
var command;

// lifecycle hooks
onMounted(() => {
  socket = new WebSocket("ws://localhost:3000/cfssession/" + route.params.cfssession + "/logs");

  socket.onmessage = (event) => {
    term.write(event.data);
  }

  term.open(document.getElementById('terminal'));

  init();
})

function init() {
  if (term._intialized) {
    return;
  }

  term._initialized = true;
  term.prompt = () => {
    term.write('\r\n$ ');
  };
  prompt(term);

  term.onData(e => {
      switch (e) {
        case '\u0003': // Ctrl+C
          term.write('^C');
          prompt(term);
          break;

        case '\r': // Enter
          runCommand(term, command);
          command = '';
          break;

        case '\u007F': // Backspace (DEL)
                       // Do not delete the prompt
          if (term._core.buffer.x > 2) {
            term.write('\b \b');
            if (command.length > 0) {
              command = command.substr(0, command.length - 1);
            }
          }

          break;

        case '\u0009':
          console.log('tabbed', output, ["dd", "ls"]);
          break;

        default:
          if (e >= String.fromCharCode(0x20) && e <= String.fromCharCode(0x7E) || e >= '\u00a0') {
            command += e;
            term.write(e);
          }
      }
  });
}

function clearInput(command) {
  var inputLengh = command.length;
  for (var i = 0; i < inputLengh; i++) {
    term.write('\b \b');
  }
}

function prompt(term) {
  command = '';
  term.write('\r\n$ ');
}

function runCommand(term, command) {
  // if (command.length > 0) {
  clearInput(command);
  socket.send(command + '\n');
  return;
  // }
}
</script>

<template>
  <div class="cfssessionlogs">
    <h1>CFS Session logs - {{ $route.params.cfssession }}</h1>
    <div id="terminal"></div>
  </div>
</template>

<style>
</style>
