<template>
  <div id="app">
    <div>
      <b-card no-body>
        <b-tabs card class="stretch">
          <b-tab active>
            <template v-slot:title>
              1. Add Lightframes
              <b-badge variant="light" v-if="$refs.lightframes.loading_exif === false">{{ $refs.lightframes.numImages }}</b-badge>
              <b-spinner type="border" small v-if="$refs.lightframes.loading_exif === true"></b-spinner>
            </template>
            <b-card-text>
              <StepDescription>Select the lightframes in this step.</StepDescription><br/>
              <ImageSelection ref="lightframes"/>
            </b-card-text>
          </b-tab>
          <b-tab>
            <template v-slot:title>
              2. Add Darkframes
              <b-badge variant="light" v-if="$refs.darkframes.loading_exif === false">{{ $refs.darkframes.numImages }}</b-badge>
              <b-spinner type="border" small v-if="$refs.darkframes.loading_exif === true"></b-spinner>
            </template>
            <b-card-text>
              <StepDescription>
                (Optional) Select darkframes, shot with the same settings as the lightframes, to reduce the noise of the resulting image.
              </StepDescription><br/>
              <ImageSelection ref="darkframes"/>
            </b-card-text>
          </b-tab>
          <b-tab title="3. Process images">
            <b-card-text>
              <StepDescription>Select processing options and start the processing.</StepDescription><br/>
              <ManageProcessing @start-processing="run_processing" ref="settings" />
            </b-card-text>
          </b-tab>
          <b-tab title="4. Preview" ref="tab_preview">
            <b-card-text>
              <Preview ref="preview" />
            </b-card-text>
          </b-tab>
        </b-tabs>
      </b-card>
    </div>


    <b-modal id="modal-readiness" hide-footer>
      <template v-slot:modal-title>
        <b-icon icon="exclamation-triangle" variant="warning"></b-icon>
        Can't start processing
      </template>

      <ul>
        <li v-if="!lightframes_ready">No lightframes are selected</li>
        <li v-if="!output_path_ready">No output path is specified</li>
      </ul>
    </b-modal>
  </div>
</template>

<script>
import ImageSelection from "@/components/ImageSelection";
import ManageProcessing from "@/components/ManageProcessing";
import { invoke } from "@tauri-apps/api/tauri";
import Preview from "@/components/Preview";
import StepDescription from "@/components/StepDescription";


export default {
  name: 'App',
  components: {
    StepDescription,
    Preview,
    ImageSelection,
    ManageProcessing
  },
  data: function () {
    return {
      lightframes_ready: false,
      output_path_ready: false
    }
  },
  methods: {
    run_processing: function () {
      let parent = this
      console.log('run', parent.$refs.settings.merge_mode, parent.$refs.settings.output_path)
      this.lightframes_ready = parent.$refs.lightframes.numImages > 1
      this.output_path_ready = parent.$refs.settings.output_path !== null

      if (!this.output_path_ready || !this.lightframes_ready) {
        this.$bvModal.show("modal-readiness")
        return
      }

      invoke("run_merge",{
        out_path: parent.$refs.settings.output_path,
        mode_str: parent.$refs.settings.merge_mode,
        lightframes: parent.$refs.lightframes.sortedImages.map(img => img.path),
        darkframes: parent.$refs.darkframes.sortedImages.map(img => img.path)
      }).then(function (preview) {
        console.log("Finished merge")
        parent.$refs.preview.preview = preview
        parent.$refs.tab_preview.activate()
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

.stretch {
  min-height: 99.5vh;
}
</style>
