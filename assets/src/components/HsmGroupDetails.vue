<script setup>

import { ref, onMounted } from 'vue'
import { useRouter, useRoute } from 'vue-router'

const router = useRouter()
const route = useRoute()

var socket;
var command;

var hsmItems = ref([])

var authToken = ref("")

const actionItems = [
  // { title: "Console", value: 1, props: { prependIcon: "mdi-console-line", onClick: (xname) => { window.open("http://localhost:5173/console/" + xname, '_blank').focus(); } } },
  { title: "Console", value: 1, props: { prependIcon: "mdi-console-line", onClick: (xname) => { router.push('/console/' + xname); } } },
  { title: "Power on", value: 2, props: { prependIcon: "mdi-power-on" } },
  { title: "Power off", value: 3, props: { prependIcon: "mdi-power-off" } }
]

// lifecycle hooks
onMounted(() => {
  authToken.value = document.cookie
    .split("; ")
    .find((row) => row.startsWith("authtoken="))
    ?.split("=")[1];

  console.log(authToken.value);
  // Get HSM groups details
  getDetails(route.params.hsm)
    .then(result => {
      hsmItems.value = result;
    });
})

async function getDetails(hsm) {
  const response = await fetch("http://localhost:3000/hsm/" + hsm, { method: "GET", headers: { "Authorization": "Bearer " + authToken.value } });

  console.log(response);

  if (response.status === 200) {
    let data = await response.json();
    data.sort(function (a, b) {
      return a.nid.localeCompare(b.nid)
    });
    return data.reverse();
  } else {
    console.error(response.statusText);
  }
}

function goToConsole(xname) {
  // window.open("http://localhost:5173/console/" + xname, '_blank').focus()
  router.push('/console/' + xname);
}

function getConsoleUrl(xname) {
  return "http://localhost:5173/console/" + xname
}
</script>

<template>
  <v-table>
    <thead>
      <tr>
        <th>XNAME</th>
        <th>NID</th>
        <th>Power Status</th>
        <th>Desired Configuration</th>
        <th>Configuration Status</th>
        <th>Enabled</th>
        <th>Error Count</th>
        <th>Boot Image ID</th>
        <th>Actions</th>
      </tr>
    </thead>
    <tbody>
      <tr v-for="item in hsmItems" :key="item.xname">
        <td><a :href="getConsoleUrl(item.xname)">{{ item.xname }}</a></td>
        <td>{{ item.nid }}</td>
        <td>{{ item.power_status }}</td>
        <td>{{ item.desired_configuration }}</td>
        <td>{{ item.configuration_status }}</td>
        <td>{{ item.enabled }}</td>
        <td>{{ item.error_count }}</td>
        <td>{{ item.boot_image_id }}</td>
        <td>
          <v-menu>
            <template v-slot:activator="{ props }">
              <v-btn icon="mdi-dots-vertical" v-bind="props" flat></v-btn>
            </template>
            <v-list>
              <v-list-item v-for="actionItem in actionItems" @click="goToConsole(item.xname)">
                <v-list-item-title>{{ actionItem.title }}</v-list-item-title>
                <template v-slot:prepend>
                  <v-icon :icon="actionItem.props.prependIcon"></v-icon>
                </template>
              </v-list-item>
            </v-list>
          </v-menu>
        </td>
      </tr>
    </tbody>
  </v-table>
</template>
<style></style>
