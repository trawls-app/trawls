<template>
  <div class="image-selection">
    <div class="d-flex justify-content-center">
      <div class="p-2"><b-button variant="success" v-on:click="choose_image_dialog">Select images</b-button></div>
      <div class="p-2"><b-button variant="warning" v-on:click="clear_list">Clear list</b-button></div>
      <div class="p-2">
        <label class="pull-right">
          <select v-model="sortkey" class="form-control">
            <option value="creation_time">Time</option>
            <option value="filename">Filename</option>
          </select>
        </label>
      </div>
    </div>

    <div class="table-responsive">
      <table class="table">
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
    clear_list: function () {
      this.images = []
      this.already_loaded = new Set()
    },
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