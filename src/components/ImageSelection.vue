<template>
  <div class="image-selection">
    <WarningCard v-if="errorWarning">
      The metadata of some files could not be loaded.
    </WarningCard>
    <br v-if="errorWarning" />
    
    <WarningCard v-if="cameraSettingWarning">
      The camera settings between some frames changed.
      Check whether the marked images really belong to the series.
    </WarningCard>
    <br v-if="cameraSettingWarning" />

    <WarningCard v-if="intervalWarning">
      Between some images, the intervals are significantly larger than the average.
      Check whether the marked images really belong to the series and if the frames are sorted correctly.
    </WarningCard>
    <br v-if="intervalWarning" />

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

    <div class="d-flex flex-row border-bottom border-light thead">
      <div class="mr-auto p-2">Filename</div>
      <div class="p-2 text-center col-medium">Exposure</div>
      <div class="p-2 text-center col-medium" v-if="showInterval">Interval</div>
      <div class="p-2 text-center col-medium">Aperture</div>
      <div class="p-2 text-center col-medium">ISO</div>
      <div class="p-2 text-center col-large">Time</div>
      <div class="p-2 col-small"></div>
    </div>

    <FrameRow
      v-for="image in sortedImages"
      :image="image"
      :setting_values="occuringSettingValues"
      :color_mapping="valueColorMapping"
      :show_interval="showInterval"
      :interval_warning_threshold="intervalWarningThreshold"
      :remove_image="remove_image"
    />
  </div>
</template>

<script>
import { open } from '@tauri-apps/api/dialog'
import { invoke } from '@tauri-apps/api/tauri'
import { listen } from '@tauri-apps/api/event'
import WarningCard from './WarningCard.vue'
import FrameRow from './FrameRow.vue'


export default {
  name: "ImageSelection",
  components: {
    WarningCard,
    FrameRow,
  },
  props: {
    showInterval: Boolean
  },
  data: function () {
    return {
      images: {},
      loading_exif: false,
      count_loaded: 0,
      interval_warning: false,
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

      // Sort by key, while moving images with errors to the front
      sorted.sort((a, b) => {
        if (a.error) return -1
        if (b.error) return 1
        return (a[this.sortkey] > b[this.sortkey]) ? 1 : -1
      })

      // Calculate intervalls between images
      let dt_prev = null
      for (let cur of sorted) {
        let dt_cur = Date.parse(cur.creation_time)

        if (dt_prev !== null && !isNaN(dt_prev)) {
          cur.interval = (dt_cur - dt_prev) / 1000 - cur.exposure_seconds
        } else {
          cur.interval = null
        }

        dt_prev = dt_cur
      }

      return sorted
    },

    /**
     * Collects all occuring aperture, exposure time and iso values
     */
    occuringSettingValues: function () {
      let data = {
        aperture: new Set(),
        exposure: new Set(),
        iso: new Set(),
      }

      for (let cur of this.sortedImages) {
        data.aperture.add(cur.aperture)
        data.exposure.add(cur.exposure_seconds)
        data.iso.add(cur.iso)
      }

      return data
    },
    
    /**
     * Maps each occuring setting value to a number from the color palette
     */
    valueColorMapping: function () {
      let data = {}

      let i = 0
      for (let [key, settings] of Object.entries(this.occuringSettingValues)) {
        data[key] = Object.fromEntries([...settings].map( setting => [setting, i++ % 8 + 1]))
      }

      return data
    },

    /**
     * Check whether there are images which differ in aperture, exposure time or iso
     */
    cameraSettingWarning: function () {
      let len_aperture = this.occuringSettingValues.aperture.size
      let len_exposure = this.occuringSettingValues.exposure.size
      let len_iso = this.occuringSettingValues.iso.size

      return len_aperture > 1 || len_exposure > 1 || len_iso > 1
    },

    /**
     * Calculate a threshold to warn about long pauses between images.
     * Returns the threshold or nothing if there are less than three images.
     */
    intervalWarningThreshold: function () {
      let interval_sum = 0

      for (let cur of this.sortedImages) {
        if (cur.interval) {
          interval_sum += cur.interval
        }
      }

      if (this.sortedImages.length > 1) {
        return interval_sum / (this.sortedImages.length - 1) + 1
      }
    },

    /**
     * Check whether the interval between two images is larger than the threshold.
     * Returns false is showInterval is false.
     */
    intervalWarning: function () {
      if (!this.showInterval) return false

      for (let cur of this.sortedImages) {
        if (Math.abs(cur.interval) > this.intervalWarningThreshold) {
          return true
        }
      }

      return false
    },

    /**
     * Check whether there are images in the list with an error.
     * Returns true on errors, otherwise false
     */
    errorWarning: function () {
      for (let cur of this.sortedImages) {
        if (cur.error !== undefined) {
          return true
        }
      }

      return false
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
    remove_image: function (path) {
      console.log("Removing", path)
      this.$delete(this.images, path)
      this.count_loaded -= 1
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
    },
    get_color_class: function (setting, value) {
      if (this.occuringSettingValues[setting].size <= 1) return "bg-palette-0"

      return "bg-palette-" + this.valueColorMapping[setting][value]
    },
  }
}
</script>

<style scoped lang="scss">
@import '../assets/darkly.scss';
table {
  margin: auto;
  width: calc(100% - 40px);
}

.col-small {
  min-width: 2em;
  max-width: 2em;
  text-align: center;
}

.col-medium {
  min-width: 5.5em;
  max-width: 5.5em;
  text-align: center;
}

.col-large {
  min-width: 12em;
  max-width: 12em;
  text-align: center;
}

.thead {
  font-weight: bolder;
}


</style>