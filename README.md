# Next-Word
Autofill text based on previous words that the model has already been trained on. You can also train the model yourself; you'll need to compile your own dataset in Dataset.json. Developed and trained by BranchRoot

Only two languages are supported by default: Russian and English. If you need the Russian version of the model, select the model in the config.json file and choose RU_Model.bin in the model_path; if you need English, choose EN_Model.bin; and if you need both Russian and English, choose Bilingual_RU_EN_Model. bin. If you need another language, communication style, or any other styles, create your own Dataset.json file, place it in the main folder, and run the program. After that, a Model.bin file will appear in the main folder—this is the new model, and you can then run it. If you want to change the Markov chain’s memory, modify the context_size parameter in Config.json. The default value is 5 If you need to increase the number of predictions, change the predict_count parameter. The default value is 1. If you need to change the path to the dataset, use the dataset_path parameter; the default is Dataset.json. If you need to change the path to the model, use the model_path parameter; the default is Model.bin.

To create your own dataset, use this template: 
[
“Phrase”,
“Phrase2” 
]
