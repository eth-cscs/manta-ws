<script setup>

import { ref, onMounted } from 'vue'
import { useRouter, useRoute } from 'vue-router'

const router = useRouter()
const route = useRoute()

var socket;
var command;

var hsmItems = ref([])

var authToken = ref("")

// MOCKING
var search = ''
var headers = [
  { key: "processors", title: "Processors" }, { key: "B", title: "B" }, { key: "C", title: "C" }
  /* {
    align: 'start',
    key: 'name',
    sortable: false,
    title: 'Dessert (100g serving)',
  },
  { key: 'calories', title: 'Calories' },
  { key: 'fat', title: 'Fat (g)' },
  { key: 'carbs', title: 'Carbs (g)' },
  { key: 'protein', title: 'Protein (g)' },
  { key: 'iron', title: 'Iron (%)' }, */
]

/* var desserts = [
  {
    name: 'Frozen Yogurt',
    calories: 159,
    fat: 6.0,
    carbs: 24,
    protein: 4.0,
    iron: 1,
  },
  {
    name: 'Ice cream sandwich',
    calories: 237,
    fat: 9.0,
    carbs: 37,
    protein: 4.3,
    iron: 1,
  },
  {
    name: 'Eclair',
    calories: 262,
    fat: 16.0,
    carbs: 23,
    protein: 6.0,
    iron: 7,
  },
  {
    name: 'Cupcake',
    calories: 305,
    fat: 3.7,
    carbs: 67,
    protein: 4.3,
    iron: 8,
  },
  {
    name: 'Gingerbread',
    calories: 356,
    fat: 16.0,
    carbs: 49,
    protein: 3.9,
    iron: 16,
  },
  {
    name: 'Jelly bean',
    calories: 375,
    fat: 0.0,
    carbs: 94,
    protein: 0.0,
    iron: 0,
  },
  {
    name: 'Lollipop',
    calories: 392,
    fat: 0.2,
    carbs: 98,
    protein: 0,
    iron: 2,
  },
  {
    name: 'Honeycomb',
    calories: 408,
    fat: 3.2,
    carbs: 87,
    protein: 6.5,
    iron: 45,
  },
  {
    name: 'Donut',
    calories: 452,
    fat: 25.0,
    carbs: 51,
    protein: 4.9,
    iron: 22,
  },
  {
    name: 'KitKat',
    calories: 518,
    fat: 26.0,
    carbs: 65,
    protein: 7,
    iron: 6,
  },
] */

// lifecycle hooks
onMounted(() => {
  authToken.value = document.cookie
    .split("; ")
    .find((row) => row.startsWith("authtoken="))
    ?.split("=")[1];

  console.log(authToken.value);
  // Get HSM groups details
  getHardware(route.params.hsm)
    .then(result => {
      hsmItems.value = result;
    });
})

async function getHardware(hsm) {
  const response = await fetch("http://localhost:3000/hsm/" + hsm + "/hardware", { method: "GET", headers: { "Authorization": "Bearer " + authToken.value } });

  if (response.status === 200) {
    let data = await response.json();
    data.sort(function (a, b) {
      return a.xname.localeCompare(b.xname)
    });
    console.log(data);
    var processors = data.flatMap((node) => node.processors.map((processor) => processor.info));
    processors = [...new Set(processors)];
    console.log("Processors:\n" + processors);
    var accelerators = data.flatMap((node) => node.node_accels.map((accelerator) => accelerator.info));
    accelerators = [...new Set(accelerators)];
    console.log("Acceleratos:\n" + accelerators);
    var memory = data.flatMap((node) => node.memory.map((memory) => memory.info));
    memory = [...new Set(memory)];
    console.log("Memory:\n" + memory);
    var header_set = new Set();
    data.forEach((node_hw) => {
      var processors = node_hw.processors.map((processor) => processor.info);
      var processor_counters = new Map();
      console.log("processor counters: " + [...processor_counters.entries()]);
      processors.forEach(function (processor) {
        console.log("processor: " + processor);
        if (processor_counters.has(processor)) {
          console.log("processor found");
          processor_counters.set(processor, processor_counters.get(processor) + 1);
          console.log("processor counters: " + [...processor_counters.entries()]);
        } else {
          console.log("processor not found");
          processor_counters.set(processor, 1);
          console.log("processor counters: " + [...processor_counters.entries()]);
        }
      })
      var accelerators = node_hw.node_accels.map((accelerator) => accelerator.info);
      var accelerator_counters = new Map();
      accelerators.forEach(function (accelerator) {
        console.log("accelerator: " + accelerator);
        if (accelerator_counters.has(accelerator)) {
          console.log("processor found");
          accelerator_counters.set(accelerator, accelerator_counters.get(accelerator) + 1);
          console.log("accelerator counters: " + [...accelerator_counters.entries()]);
        } else {
          console.log("accelerator not found");
          accelerator_counters.set(accelerator, 1);
          console.log("accelerator counters: " + [...accelerator_counters.entries()]);
        }
      })
      var memory_dimms = node_hw.memory.map((memory) => memory.info);
      var accelerator_counters = new Map();
      memory_dimms.forEach(function (memory) {
        console.log("memory: " + memory);
        if (memory_counters.has(memory)) {
          console.log("processor found");
          memory_counters.set(memory, memory_counters.get(memory) + 1);
          console.log("memory counters: " + [...memory_counters.entries()]);
        } else {
          console.log("memory not found");
          memory_counters.set(memory, 1);
          console.log("memory counters: " + [...memory_counters.entries()]);
        }
      })

      var node = {};
      node.xname = node_hw.xname;
      node.processors = processor_counters;
      console.log("Node: " + JSON.stringify(node));
    });
    headers = processors + accelerators + memory;
    console.log("Headers: " + headers);
    return data;
  } else {
    console.error(response.statusText);
  }
}

</script>

<template>
  <v-card flat title="Filter">
    <template v-slot:text>
      <v-text-field v-model="search" label="Search" prepend-inner-icon="mdi-magnify" single-line variant="outlined"
        hide-details></v-text-field>
    </template>

    <v-data-table :headers="headers" :items="hsmItems" :search="search"></v-data-table>
  </v-card>

  <!-- <v-table>
    <thead>
      <tr>
        <th>XNAME</th>
        <th>NID</th>
        <th>Power Status</th>
        <th>Desired Configuration</th>
        <th>Configuration Status</th>
        <th>Enabled</th>
        <th>Error Count</th>
        <th>Boot configuration</th>
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
        <td>{{ item.boot_configuration }}</td>
        <td>{{ item.boot_image_id }}</td>
        <td>
          <v-menu>
            <template v-slot:activator="{ props }">
              <v-btn icon="mdi-dots-vertical" v-bind="props" flat></v-btn>
            </template>
            <v-list>
              <v-list-item v-for="actionItem in actionItems" @click="actionItem.doIt(item.xname)">
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
  </v-table> -->
</template>
<style></style>
