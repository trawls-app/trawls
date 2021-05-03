<template>
  <div>
    <b-img v-bind:src="'data:image/jpeg;base64,'+image_data" fluid v-if="image_data" />
  </div>
</template>

<script>

export default {
  name: "Preview",
  data: function () {
    return {
      image_data: null
    }
  }
}
</script>

<style scoped>

</style>