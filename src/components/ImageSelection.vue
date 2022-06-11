<template>
  <div class="image-selection">
    <WarningCard>
      The metadata of some files could not be loaded.
    </WarningCard>
    <br />

    <WarningCard v-if="interval_warning && showInterval">
      Between some images, the intervals are significantly larger than the average.
      Check whether the marked images really belong to the series and if the frames are sorted correctly.
    </WarningCard>
    <br />

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
          <td class="text-center">Exposure</td>
          <td class="text-center" v-if="showInterval">Interval</td>
          <td class="text-center">Aperture</td>
          <td class="text-center">ISO</td>
          <td class="text-center">Time</td>
          <td></td>
        </tr>
        </thead>
        <tbody>
        <tr v-for="(image, path) in sortedImages" :key="path" :class="{ 'bg-danger': image.error }">
          <td v-b-tooltip.hover="image.path">{{ image.filename}}</td>
          <td colspan="5" v-if="image.error">
            <b-icon icon="patch-exclamation"></b-icon>
            {{ image.error }}
          </td>
          <td class="text-center" v-if="!image.error">{{ image.exposure_seconds }}s</td>
          <td class="text-center" v-if="!image.error && showInterval" :class="{ 'bg-warning': Math.abs(image.interval) > interval_warning_threshold}">
            <span v-if="image.interval !== null">{{ image.interval }}s</span>
          </td>
          <td class="text-center" v-if="!image.error">f{{ image.aperture }}</td>
          <td class="text-center" v-if="!image.error">{{ image.iso }}</td>
          <td class="text-center" v-if="!image.error">{{ image.creation_time}}</td>
          <td>
            <b-icon class="clickable-icon" icon="x-circle" v-on:click="remove_image(image.path)"></b-icon>
          </td>
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
import WarningCard from './WarningCard.vue'


export default {
  name: "ImageSelection",
  components: {
    WarningCard,
  },
  props: {
    showInterval: Boolean
  },
  data: function () {
    return {
      images: {},
      loading_exif: false,
      count_loaded: 0,
      interval_warning_threshold: 2,
      interval_warning: false,
      error_warning: false,
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

      sorted.sort((a, b) => {
        if (a.error) return -1
        if (b.error) return 1
        return (a[this.sortkey] > b[this.sortkey]) ? 1 : -1
      })

      // Calculate intervalls between images
      let dt_prev = null
      let interval_sum = 0
      let max_interval = 0

      for (let cur of sorted) {
        let dt_cur = Date.parse(cur.creation_time)

        if (dt_prev !== null) {
          cur.interval = (dt_cur - dt_prev) / 1000 - cur.exposure_seconds
          interval_sum += cur.interval
          max_interval = Math.max(Math.abs(cur.interval), max_interval)
        } else {
          cur.interval = null
        }

        if (cur.error !== undefined) {
          this.error_warning = true
        }

        dt_prev = dt_cur
      }

      // Calculate a threshold to warn about long pauses between images
      if (sorted.length > 1) {
        this.interval_warning_threshold = interval_sum / (sorted.length - 1) + 1
        console.log("Set interval warning threshold", this.interval_warning_threshold)
      }

      // Check whether any image exceeded the max interval
      this.interval_warning = max_interval > this.interval_warning_threshold

      return sorted
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
      this.interval_warning = false
      this.error_warning = false
    },
    remove_image: function (path) {
      console.log("Removing", path)
      this.$delete(this.images, path)
      this.count_loaded -= 1
      this.error_warning = false
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

.clickable-icon {
  cursor: pointer;
}
</style>