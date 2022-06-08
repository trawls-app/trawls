<template>
  <div class="image-selection">
    <div class="d-flex justify-content-center">
      <div class="p-2"><b-button variant="success" v-on:click="choose_image_dialog">Select images</b-button></div>
      <div class="p-2"><b-button variant="warning" v-on:click="clear_list">Clear list</b-button></div>
      <div class="p-2"><b-form-select v-model="sortkey" :options="available_sortkeys"></b-form-select></div>
    </div>

    <b-progress class="mt-2" :max="numImages" v-if="loading_exif">
      <b-progress-bar :value="count_loaded" variant="success">
        <span><strong>{{ count_loaded }} / {{ numImages }}</strong></span>
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
import { open } from '@tauri-apps/api/dialog'
import { invoke } from '@tauri-apps/api/tauri'
import { listen } from '@tauri-apps/api/event'

let vue = undefined

export default {
  name: "ImageSelection",
  data: function () {
    return {
      images: {},
      loading_exif: false,
      count_loaded: 0,
      sortkey: 'creation_time',
      available_sortkeys: [
          { value: 'creation_time', text: 'Time' },
          { value: 'filename', text: 'Filename'}
      ]
    }
  },
  created() {
    listen('loaded_image_info_' + this._uid, payload => { this.set_image_infos(payload.payload) })
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
      this.count_loaded = 0
    },
    choose_image_dialog: function (event) {
      console.log(this._uid)
      let parent = this
      if (event) {
        open({multiple: true}).then(function (res) {
          parent.loading_exif = true

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
    set_image_infos: function (infos) {
      this.count_loaded += Object.keys(infos.image_infos).length

      for (let [path, info] of Object.entries(infos.image_infos)) {
        this.$set(this.images, path, info)
      }

      if (infos.count_loaded === infos.count_total) {
        this.loading_exif = false
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