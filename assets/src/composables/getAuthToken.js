import { ref, onMounted } from 'vue'

// by convention, composable function names start with "use"
export function cookieUtils() {
  // state encapsulated and managed by the composable
  const authToken = ref("")

  // a composable can update its managed state over time.
  function getAuthToken() {
    authToken.value =  getCookieByName("authtoken")
  }

  function getCookieByName(cookieName) {
    return document.cookie
    .split("; ")
    .find((row) => row.startsWith(cookieName + "="))
    ?.split("=")[1];
  }

  // a composable can also hook into its owner component's
  // lifecycle to setup and teardown side effects.
  onMounted(() => getAuthToken

  // expose managed state as return value
  return { getAuthToken }
}
