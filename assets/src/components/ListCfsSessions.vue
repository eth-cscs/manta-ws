<script setup>

import { onMounted } from 'vue'
import { useRouter, useRoute } from 'vue-router'

import get from './CfsSsesionHttpClient.js'

const router = useRouter()
const route = useRoute()

var socket;
var command;

var items = [/* {title: "1"}, {title: "2"} */]

// lifecycle hooks
onMounted(() => {

  // Get HSM groups details
  items = get("psi-tds");
})
</script>

<template>
  <v-list lines="one">
    <v-list-item
    v-for="item in items"
    :key="item.name"
    :title="item.name"
    subtitle="Subtitle"
    ></v-list-item>
  </v-list>
</template>
<style>
</style>

