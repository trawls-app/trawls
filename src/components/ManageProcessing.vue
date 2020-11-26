<template>
  <b-button v-on:click="$emit('start-processing')">Process images</b-button>
</template>

<script>
export default {
  name: "ManageProcessing",
  emits: {
    'start-processing': null
  }
}
</script>

<style scoped>

</style>