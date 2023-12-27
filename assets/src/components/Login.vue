<script setup>

import { ref } from 'vue'

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

var loading = ref(false)

async function submit() {
  loading.value = true;
  const response = await fetch("http://localhost:3000/authenticate", { method: "GET", headers: { "Authorization": "Basic " + btoa(username.value + ":" + password.value) } });

  console.log(response);

  if (response.status === 200) {
    let data = await response.text();
    // Store auth jwt token in cookie "authtoken"
    document.cookie = "authtoken=" + data + "; SameSite=None; Secure";
    router.push('/hsm')
    // window.open("http://localhost:5173/hsm", '_blank').focus();
  } else {
    console.error(response.statusText);
  }

  loading.value = false;
}

</script>

<template>
  <v-card :loading="loading" max-width="500" align="center">
    <v-img cover height="250" src="https://www.cscs.ch/fileadmin/_processed_/5/b/csm_Alps_page_675f9f8307.png"></v-img>
    <template v-slot:loader="{ isActive }">
      <v-progress-linear :active="isActive" color="deep-purple" height="4" indeterminate></v-progress-linear>
    </template>
    <v-card-item>
      <v-card-title>Welcome to Alps infastructure</v-card-title>
      <v-card-subtitle>Please login</v-card-subtitle>
      <v-sheet width="500" class="mx-auto">
        <v-form @submit.prevent="submit">
          <v-text-field v-model="username" :rules="rules" label="Username"></v-text-field>
          <v-text-field v-model="password" type="password" :rules="rules" label="Password"></v-text-field>
          <v-btn type="submit" color="primary" class="mt-2">Submit</v-btn>
        </v-form>
      </v-sheet>
    </v-card-item>
  </v-card>
</template>
