<template>
  <b-card bg-variant="default" border-variant="warning">
    <b-card-text>
        <div class="d-flex">
            <div class="p-2"><b-icon icon="patch-exclamation" font-scale="2.5" class="text-warning"></b-icon></div>
            <div class="p-2"><slot></slot></div>
        </div>
    </b-card-text>
  </b-card>
</template>

<script>
export default {
  name: "WarningCard"
}
</script>

<style scoped>

</style>