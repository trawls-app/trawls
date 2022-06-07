<template>
  <div class="image-selection">
    <div class="d-flex justify-content-center">
      <div class="p-2"><b-button variant="success" v-on:click="choose_image_dialog">Select images</b-button></div>
      <div class="p-2"><b-button variant="warning" v-on:click="clear_list">Clear list</b-button></div>
      <div class="p-2"><b-form-select v-model="sortkey" :options="available_sortkeys"></b-form-select></div>
    </div>

    <b-progress class="mt-2" :max="sortedImages.length">
      <b-progress-bar :value="images.length - loading_exif" variant="success">
        <span><strong>{{ images.length - loading_exif }} / {{ sortedImages.length }}</strong></span>
      </b-progress-bar>
    </b-progress>

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
        <tr v-for="(image, path) in images" :key="path">
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
import { open } from '@tauri-apps/api/dialog'
import { invoke } from '@tauri-apps/api/tauri'
import { listen } from '@tauri-apps/api/event'

let vue = undefined

export default {
  name: "ImageSelection",
  data: function () {
    return {
      images: {},
      loading_exif: 0,
      sortkey: 'creation_time',
      available_sortkeys: [
          { value: 'creation_time', text: 'Time' },
          { value: 'filename', text: 'Filename'}
      ]
    }
  },
  created() {
    listen('loaded_image_info_' + this._uid, payload => { this.set_image_info(payload) })
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
      console.log(this._uid)
      let parent = this
      if (event) {
        open({multiple: true}).then(function (res) {
          parent.loading_exif = res.length

          // Show images immediately
          for (let image of res) {
            parent.$set(parent.images, image, {"filename": image.split("/").pop()})
          }

          invoke("load_image_infos",{
            paths: res,
            selectorReference: parent._uid.toString()
          }).catch(error => { alert(error) })
        })
      }
    },
    set_image_info: function (info) {
      this.$set(this.images, info.payload.path, info.payload)
      this.loading_exif -= 1
      console.log(this.loading_exif)
      //this.$forceUpdate()
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