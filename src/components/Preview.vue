<template>
  <div v-if="preview">
    <image-zoom
        v-bind:regular="'data:image/jpeg;base64,'+preview.encoded"
        img-class="img-fluid" v-bind:zoom-amount="3" click-zoom>
    </image-zoom>
    <small>{{ preview.isospeed }}, {{ preview.aperture }}, {{ preview.exposure }}</small>
  </div>
  <div v-else>
    <StepDescription>There is no preview, because no images have been processed yet.</StepDescription>
  </div>
</template>

<script>

import StepDescription from "@/components/StepDescription";
export default {
  name: "Preview",
  components: {StepDescription},
  data: function () {
    return {
      preview: null
    }
  }
}
</script>

<style scoped>

</style>