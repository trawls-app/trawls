<template>
  <div class="image-selection">
    <button v-on:click="choose_image_dialog">Add images</button>
    <select v-model="sortkey">
      <option value="creation_time">Time</option>
      <option value="filename">Filename</option>
    </select>
    <button v-on:click="run_processing">Process images</button>

    <table>
      <thead>
        <tr>
          <td>Filename</td>
          <td>ISO</td>
          <td>Width</td>
          <td>Height</td>
          <td>Time</td>
        </tr>
      </thead>
      <tbody>
        <tr v-for="image in sortedImages" :key="image.path">
          <td>{{ image.filename}}</td>
          <td>{{ image.iso }}</td>
          <td>{{ image.width}}</td>
          <td>{{ image.height }}</td>
          <td>{{ image.creation_time}}</td>
        </tr>
      </tbody>
    </table>
  </div>
</template>

<script>
import { open } from 'tauri/api/dialog'
import { promisified } from 'tauri/api/tauri'

export default {
  name: "ImageSelection",
  data: function () {
    return {
      images: [],
      already_loaded: new Set(),
      sortkey: 'creation_time'
    }
  },
  computed: {
    sortedImages: function () {
      let sorted = [...this.images]
      return sorted.sort((a, b) => (a[this.sortkey] > b[this.sortkey]) ? 1 : -1)
    }
  },
  methods: {
    choose_image_dialog: function (event) {
      let parent = this
      if (event) {
        open({multiple: true}).then(function (res) {
          for (let image of res) {
            if (parent.already_loaded.has(image)) { continue }
            promisified({
              cmd: "loadImage",
              path: image
            }).then(function (resp) {
              if (!parent.already_loaded.has(resp.path)) {
                parent.images.push(resp)
                parent.already_loaded.add(resp.path)
              }
            })
          }
        })
      }
    },
    run_processing: function (event) {
      let parent = this
      if (event) {
        promisified({
          cmd: "runMerge",
          mode_str: "normal",
          lightframes: parent.sortedImages.map(img => img.path)
        }).then(function () {
          parent.images = []
          parent.already_loaded = new Set()
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
  font-weight: bold;
}
</style>