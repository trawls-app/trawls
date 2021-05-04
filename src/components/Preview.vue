<template>
  <div v-if="preview">
    <image-zoom
        v-bind:regular="'data:image/jpeg;base64,'+preview.encoded"
        img-class="img-fluid" v-bind:zoom-amount="3" click-zoom>
    </image-zoom>
    <small>{{ preview.isospeed }}, {{ preview.aperture }}, {{ preview.exposure }}</small>
  </div>
  <div v-else>
    <b-card class="text-center" bg-variant="default" text-variant="white" border-variant="success">
      <b-card-text>
        There is no preview, because no images have been processed yet.
      </b-card-text>
    </b-card>
  </div>
</template>

<script>

export default {
  name: "Preview",
  data: function () {
    return {
      preview: null
    }
  }
}
</script>

<style scoped>

</style>