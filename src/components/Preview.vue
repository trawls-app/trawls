<template>
  <div>
    <image-zoom
        v-bind:regular="'data:image/jpeg;base64,'+image_data" v-if="image_data"
        img-class="img-fluid" v-bind:zoom-amount="3" click-zoom>
    </image-zoom>
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