WITH RankedPosts AS (
    SELECT 
        p.Id AS PostId,
        p.Title,
        p.CreationDate,
        p.Score,
        p.ViewCount,
        u.DisplayName AS OwnerDisplayName,
        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
    FROM 
        Posts p
    JOIN 
        Users u ON p.OwnerUserId = u.Id
    WHERE 
        p.CreationDate > cast('2024-10-01' as date) - INTERVAL '30 days'
        AND p.Score > 10
), 
PostAnswers AS (
    SELECT 
        p.Id AS QuestionId,
        COUNT(a.Id) AS AnswerCount
    FROM 
        Posts p
    LEFT JOIN 
        Posts a ON p.Id = a.ParentId AND a.PostTypeId = 2
    WHERE 
        p.PostTypeId = 1
    GROUP BY 
        p.Id
),
PostHistoryCTE AS (
    SELECT 
        ph.PostId,
        COUNT(ph.Id) AS EditCount,
        MAX(ph.CreationDate) AS LastEditDate,
        STRING_AGG(DISTINCT CONCAT('Type: ', pht.Name, ' by ', ph.UserDisplayName), '; ' ORDER BY CONCAT('Type: ', pht.Name, ' by ', ph.UserDisplayName)) AS EditDetails
    FROM 
        PostHistory ph
    JOIN 
        PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id
    GROUP BY 
        ph.PostId
)

SELECT 
    rp.PostId,
    rp.Title,
    rp.CreationDate,
    rp.Score,
    COALESCE(pa.AnswerCount, 0) AS AnswerCount,
    rp.ViewCount,
    rp.OwnerDisplayName,
    ph.EditCount,
    ph.LastEditDate,
    ph.EditDetails
FROM 
    RankedPosts rp
LEFT JOIN 
    PostAnswers pa ON rp.PostId = pa.QuestionId
LEFT JOIN 
    PostHistoryCTE ph ON rp.PostId = ph.PostId
WHERE 
    rp.Rank <= 5
ORDER BY 
    rp.Score DESC, 
    rp.CreationDate DESC;