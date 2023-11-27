<script setup>
import HsmGroupDetails from '../components/HsmGroupDetails.vue'
</script>

<template>
<main>
<div :class="['text-h2']">HSM {{ $route.params.hsm }}</div>
<HsmGroupDetails />
</main>
</template>
