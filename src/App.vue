<template>
  <div id="app">
    <div>
      <b-card no-body>
        <b-tabs card>
          <b-tab title="Lightframes" active>
            <b-card-text><ImageSelection ref="lightframes"/></b-card-text>
          </b-tab>
          <b-tab title="Darkframes" disabled>
            <b-card-text><ImageSelection ref="darkframes"/></b-card-text>
          </b-tab>
          <b-tab title="Process">
            <b-card-text><ManageProcessing @start-processing="run_processing"/></b-card-text>
          </b-tab>
        </b-tabs>
      </b-card>
    </div>
  </div>
</template>

<script>
import ImageSelection from "@/components/ImageSelection";
import ManageProcessing from "@/components/ManageProcessing";
import {promisified} from "tauri/api/tauri";


export default {
  name: 'App',
  components: {
    ImageSelection,
    ManageProcessing
  },
  methods: {
    run_processing: function () {
      console.log('run')
      let parent = this
      promisified({
        cmd: "runMerge",
        mode_str: "normal",
        lightframes: parent.$refs.lightframes.sortedImages.map(img => img.path)
      }).then(function () {
        parent.$refs.lightframes.images = []
        parent.$refs.lightframes.already_loaded = new Set()
      })
    }
  }
}
</script>

<style lang="scss">
@import 'assets/darkly.scss';

// Bootstrap and its default variables
@import '../node_modules/bootstrap/scss/bootstrap';
// BootstrapVue and its default variables
@import '../node_modules/bootstrap-vue/src/index.scss';
</style>
