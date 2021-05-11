<template>
  <div class="image-selection">
    <div class="d-flex justify-content-center">
      <div class="p-2"><b-button variant="success" v-on:click="choose_image_dialog">Select images</b-button></div>
      <div class="p-2"><b-button variant="warning" v-on:click="clear_list">Clear list</b-button></div>
      <div class="p-2"><b-form-select v-model="sortkey" :options="available_sortkeys"></b-form-select></div>
    </div>

    <div class="table-responsive">
      <table class="table" v-if="sortedImages.length > 0">
        <thead>
        <tr>
          <td>Filename</td>
          <td class="text-center">Aperture</td>
          <td class="text-center">Exposure</td>
          <td class="text-center">ISO</td>
          <td class="text-center">Time</td>
        </tr>
        </thead>
        <tbody>
        <tr v-for="(image, path) in sortedImages" :key="path">
          <td>{{ image.filename}}</td>
          <td class="text-center">f{{ image.aperture }}</td>
          <td class="text-center">{{ image.exposure_seconds }}s</td>
          <td class="text-center">{{ image.iso }}</td>
          <td class="text-center">{{ image.creation_time}}</td>
        </tr>
        </tbody>
      </table>
    </div>
  </div>
</template>

<script>
import { open } from 'tauri/api/dialog'
import { promisified } from 'tauri/api/tauri'

export default {
  name: "ImageSelection",
  data: function () {
    return {
      images: {},
      loading_exif: false,
      sortkey: 'creation_time',
      available_sortkeys: [
          { value: 'creation_time', text: 'Time' },
          { value: 'filename', text: 'Filename'}
      ]
    }
  },
  computed: {
    sortedImages: function () {
      let sorted = [...Object.values(this.images)]
      return sorted.sort((a, b) => (a[this.sortkey] > b[this.sortkey]) ? 1 : -1)
    },
    numImages: function () {
      return Object.keys(this.images).length
    },
    ready: function () {
      return this.numImages > 1 && !this.loading_exif
    }
  },
  methods: {
    clear_list: function () {
      this.images = {}
    },
    choose_image_dialog: function (event) {
      let parent = this
      if (event) {
        open({multiple: true}).then(function (res) {
          parent.loading_exif = true

          // Show images immediately
          for (let image of res) {
            parent.$set(parent.images, image, {"filename": image.split("/").pop()})
          }

          promisified({
            cmd: "loadImages",
            paths: res
          }).then(function (resp) {
            for (let r of resp) {
              parent.$set(parent.images, r.path, r)
            }
            parent.loading_exif = false
          })
        })
      }
    }
  }
}
</script>

<style scoped>
table {
  margin: auto;
  width: calc(100% - 40px);
}

thead {
  font-weight: bolder;
}
</style>