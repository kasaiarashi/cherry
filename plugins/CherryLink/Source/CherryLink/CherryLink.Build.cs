// Copyright CherryLink Contributors. All Rights Reserved.

using UnrealBuildTool;

public class CherryLink : ModuleRules
{
	public CherryLink(ReadOnlyTargetRules Target) : base(Target)
	{
#if UE_4_22_OR_LATER
		PCHUsage = PCHUsageMode.NoPCHs;
#else
		PCHUsage = PCHUsageMode.NoSharedPCHs;
#endif

#if UE_5_2_OR_LATER
		bDisableStaticAnalysis = true;
#endif

		PublicDependencyModuleNames.AddRange(new string[]
		{
			"Core",
			"CoreUObject",
			"Engine",
			"Sockets",
			"Networking",
			"Json",
			"JsonUtilities"
		});

		PrivateDependencyModuleNames.AddRange(new string[]
		{
			"UnrealEd",
			"Slate",
			"SlateCore",
			"EditorStyle",
			"LevelEditor",
			"Kismet",
			"BlueprintGraph",
			"AssetRegistry",
			"ToolMenus",
			"EditorSubsystem",
			"Projects",
			"SourceCodeAccess"
		});
	}
}
