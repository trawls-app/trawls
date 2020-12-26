<template>
  <div id="app">
    <div>
      <b-card no-body>
        <b-tabs card>
          <b-tab title="1. Add Lightframes" active>
            <b-card-text><ImageSelection ref="lightframes"/></b-card-text>
          </b-tab>
          <b-tab title="2. Add Darkframes" disabled>
            <b-card-text><ImageSelection ref="darkframes"/></b-card-text>
          </b-tab>
          <b-tab title="3. Process images">
            <b-card-text><ManageProcessing @start-processing="run_processing" ref="settings"/></b-card-text>
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
      let parent = this
      console.log('run', parent.$refs.merge_mode)
      promisified({
        cmd: "runMerge",
        mode_str: parent.$refs.settings.merge_mode,
        lightframes: parent.$refs.lightframes.sortedImages.map(img => img.path)
      }).then(function () {
        parent.$refs.lightframes.clear_list()
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
