<script setup>

import { ref, onMounted } from 'vue'
import { useRouter, useRoute } from 'vue-router'

const router = useRouter()
const route = useRoute()

var hsmItems = ref([])
var authToken = ref("")

// lifecycle hooks
onMounted(() => {
  authToken.value = document.cookie
    .split("; ")
    .find((row) => row.startsWith("authtoken="))
    ?.split("=")[1];

  console.log(authToken.value);

  getHsmSummary();
})

async function getHsmSummary() {
  const response = await fetch("http://localhost:3000/hsm", { method: "GET", headers: { "Authorization": "Bearer " + authToken.value } });

  console.log(response);

  if (response.status === 200) {
    let data = await response.json();
    hsmItems.value = data;
  } else {
    console.log("ERROR - " + data);
    console.error(response.statusText);
  }
}

</script>

<template>
  <v-row align="center" justify="center">
    <v-col v-for="hsmItem in hsmItems" :key="hsmItem.label" class="d-flex child-flex" cols="2">
      <v-card class="mx-auto" @click="router.push('/hsm/' + hsmItem.label)" max-width="344" :title="hsmItem.label"
        :subtitle="hsmItem.description" prepend-icon="mdi-server-network" append-icon="mdi-check">
      </v-card>
    </v-col>
  </v-row>
</template>
<style></style>
