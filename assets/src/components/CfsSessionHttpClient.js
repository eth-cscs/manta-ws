import { ref } from 'vue'

export default async function get() {

  // Call Cama http to get hsm details
  const response = await fetch("http://localhost:3000/cfssessions", {method: "GET"});
    // .then((response) => response.json())
    // .then((json) => console.log(json.json()));

  console.log(response);

  if (response.status === 200) {
    let data = await response.json();
    console.log(data);
    return data;
  } else {
    console.error(response.statusText);
  }
}
