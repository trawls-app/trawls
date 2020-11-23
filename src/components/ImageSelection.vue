<template>
  <div class="image-selection">
    <button v-on:click="choose_image_dialog">Choose images</button>
    <ul id="file-list">
      <li v-for="image in images" :key="image.id">{{ image }}</li>
    </ul>
  </div>
</template>

<script>
import { open } from 'tauri/api/dialog'
import { promisified } from 'tauri/api/tauri'

export default {
  name: "ImageSelection",
  data: function () {
    return {
      images: []
    }
  },
  methods: {
    choose_image_dialog: function (event) {
      let parent = this
      if (event) {
        open({multiple: true}).then(function (res) {
          for (let image of res) {
            promisified({
              cmd: "loadImage",
              path: image
            }).then(function (resp) {
              parent.images.push(resp)
            })
          }
        })
      }
    }
  }
}
</script>

<style scoped>

</style>