<script setup>
import HsmGroupHardware from '../components/HsmGroupHardware.vue'
</script>

<template>
  <main>
    <div :class="['text-h2']">HSM {{ $route.params.hsm }} hardware details</div>
    <v-divider inset></v-divider>
    <HsmGroupHardware />
  </main>
</template>
