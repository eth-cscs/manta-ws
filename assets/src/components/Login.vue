<script setup>

import { ref, onMounted } from 'vue'
import { useRouter, useRoute } from 'vue-router'

const router = useRouter()
const route = useRoute()

var rules = ref([
value => {
if (value) return true
return 'Please enter a value'
}
])

var username = ref("")
var password = ref("")

async function submit() {
  alert("Login!!!!");
}


</script>

<template>
<v-card>
  :loading="loading"
  max-width="500"
  >
  <template v-slot:loader="{ isActive }">
    <v-progress-linear :active="isActive" color="deep-purple" height="4" indeterminate></v-progress-linear>
  </template>
  <v-img cover height="250" src="https://cdn.vuetifyjs.com/images/cards/cooking.png"></v-img>
  <v-card-item>
    <v-card-title>Wellcome to Alps infastructure</v-card-title>
    <v-card-subtitle>Please login</v-card-subtitle>
    <v-sheet width="500" class="mx-auto">
      <v-form @submiut.prevent="login">
      <v-text-field v-model="username" :rules="rules" label="Username"></v-text-field>
      <v-text-field v-model="password" type="password" :rules="rules" label="Password"></v-text-field>
      <v-btn type="submit" @click="submit" color="primary" class="mt-2">Submit</v-btn>
      </v-form>
    </v-sheet>
  </v-card-item>
</v-card>
</template>
