<template>
  <div v-if="preview" class="text-center">
    <image-zoom
        v-bind:regular="'data:image/jpeg;base64,'+preview.encoded"
        img-class="preview-fluid" v-bind:zoom-amount="3" click-zoom>
    </image-zoom><br>
    <small>{{ preview.isospeed }}, {{ preview.aperture }}, {{ preview.exposure }}</small>
  </div>
  <div v-else>
    <StepDescription>There is no preview, because no images have been processed yet.</StepDescription>
  </div>
</template>

<script>

import StepDescription from "@/components/StepDescription";
export default {
  name: "ImagePreview",
  components: {StepDescription},
  data: function () {
    return {
      preview: null
    }
  }
}
</script>

<style>

.preview-fluid {
  max-width: 100%;
  max-height: 92vh;
  height: auto;
}
</style>